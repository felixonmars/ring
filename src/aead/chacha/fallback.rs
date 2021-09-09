// Copyright 2021 Brian Smith.
// Portions Copyright (c) 2014, Google Inc.
//
// Permission to use, copy, modify, and/or distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY
// SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION
// OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN
// CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE. */
// Adapted from the public domain, estream code by D. Bernstein.
// Adapted from the BoringSSL crypto/chacha/chacha.c.

use super::{CounterOrIv, Key, BLOCK_LEN};
use crate::endian::*;
use crate::polyfill::ChunksFixedMut;

pub(super) fn ChaCha20_ctr32(
    key: &Key,
    counter: CounterOrIv,
    mut input: *const u8,
    mut in_out_len: usize,
    mut output: *mut u8,
) {
    const SIGMA: [u32; 4] = [
        u32::from_le_bytes(*b"expa"),
        u32::from_le_bytes(*b"nd 3"),
        u32::from_le_bytes(*b"2-by"),
        u32::from_le_bytes(*b"te k"),
    ];

    let key = key.words_less_safe();
    // let counter = counter.into_words_less_safe();
    let iv = match counter {
        CounterOrIv::Counter(counter) => counter.into(),
        CounterOrIv::Iv(iv) => {
            assert!(in_out_len <= 32);
            iv
        }
    };
    let counter = <[LittleEndian::<u32>;4]>::from_byte_array(&iv.0);

    let mut state = [
        SIGMA[0], SIGMA[1], SIGMA[2], SIGMA[3], key[0].into(), key[1].into(), key[2].into(), key[3].into(), key[4].into(), key[5].into(),
        key[6].into(), key[7].into(), counter[0].into(), counter[1].into(), counter[2].into(), counter[3].into(),
    ];

    // let mut in_out_len = in_out.len().checked_sub(src.start).unwrap();
    // let mut input = in_out[src].as_ptr();
    // let mut output = in_out.as_mut_ptr();

    let mut buf = [0u8; BLOCK_LEN];
    while in_out_len > 0 {
        chacha_core(&mut buf, &state);
        state[12] += 1;

        let todo = core::cmp::min(BLOCK_LEN, in_out_len);
        for (i, &b) in buf[..todo].iter().enumerate() {
            let input = unsafe { *input.add(i) };
            let b = input ^ b;
            unsafe { *output.add(i) = b };
        }

        in_out_len -= todo;
        input = unsafe { input.add(todo) };
        output = unsafe { output.add(todo) };
    }
}

// Performs 20 rounds of ChaCha on `input`, storing the result in `output`.
#[inline(always)]
fn chacha_core(output: &mut [u8; BLOCK_LEN], input: &State) {
    let mut x = *input;

    for _ in (0..20).step_by(2) {
        quarterround(&mut x, 0, 4, 8, 12);
        quarterround(&mut x, 1, 5, 9, 13);
        quarterround(&mut x, 2, 6, 10, 14);
        quarterround(&mut x, 3, 7, 11, 15);
        quarterround(&mut x, 0, 5, 10, 15);
        quarterround(&mut x, 1, 6, 11, 12);
        quarterround(&mut x, 2, 7, 8, 13);
        quarterround(&mut x, 3, 4, 9, 14);
    }

    for (x, input) in x.iter_mut().zip(input.iter()) {
        *x = x.wrapping_add(*input);
    }

    for (output, &x) in ChunksFixedMut::<[u8; 4]>::chunks_fixed_mut(output).zip(x.iter()) {
        *output = u32::to_le_bytes(x)
    }
}

#[inline(always)]
fn quarterround(x: &mut State, a: usize, b: usize, c: usize, d: usize) {
    #[inline(always)]
    fn step(x: &mut State, a: usize, b: usize, c: usize, rotation: u32) {
        x[a] = x[a].wrapping_add(x[b]);
        x[c] = (x[c] ^ x[a]).rotate_left(rotation);
    }
    step(x, a, b, d, 16);
    step(x, c, d, b, 12);
    step(x, a, b, d, 8);
    step(x, c, d, b, 7);
}

type State = [u32; BLOCK_LEN];
