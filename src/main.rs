use std::io::{stdin, stdout, Write};

fn main() {
    let mut input = String::new();
    print!("Input: ");
    let _ = stdout().flush();
    stdin()
        .read_line(&mut input)
        .expect("Did not enter a correct string");

    let mut bytes = input.trim().as_bytes().to_vec();

    // fail-safe für leeren Input
    if bytes.is_empty() {
        bytes.push(0x00); // minimaler Dummy-Byte
    }
    let mut state = [147u8; 64]; // initial state

    // padding / input length influence
    state[0] ^= bytes.len() as u8;
    state[1] ^= (bytes.len() >> 8) as u8;

    // init block
    let mut block = [0u8; 64];
    let len = bytes.len().min(64);
    block[..len].copy_from_slice(&bytes[..len]);

    let mut sbox = [0u8; 256];
    for i in 0..256 {
        sbox[i] = i as u8;
    }

    let mut seed: u8 = bytes[0].wrapping_add(bytes[bytes.len() - 1]);

    for i in (1..256).rev() {
        seed = seed.wrapping_add(bytes[i % len])
            ^ (seed % i as u8).wrapping_add(sbox[i as usize])
            ^ (sbox[i] ^ sbox[(i + 7) % 256])
            ^ (seed.rotate_left((i % 5) as u32).wrapping_add(sbox[i]));
        let j = ((seed as usize)
            ^ (i ^ (seed
                .rotate_left((i % 5) as u32)
                .wrapping_add(sbox[i].wrapping_add(sbox[i]))) as usize)
                .wrapping_add(i))
            % 256;

        // XOR mit Input
        sbox[i] ^= bytes[i % len];
        sbox[j] ^= bytes[(i + 1) % len];

        sbox.swap(i, j);
    }

    for i in (1..256).rev() {
        seed ^= sbox[(i * 3) % 256];
        seed = seed.wrapping_add(bytes[(i + 3) % len])
            ^ (seed % i as u8).wrapping_add(sbox[i])
            ^ (sbox[i] ^ sbox[(i + 13) % 256])
            ^ (seed
                .rotate_left((i % 7) as u32)
                .wrapping_add(sbox[(i + 5) % 256]));
        let j = ((seed as usize) ^ i) % 256;

        sbox[i] ^= bytes[(i + 16) % len];
        sbox[j] ^= bytes[(i + (i.wrapping_mul(11) ^ i)) % len];
        sbox.swap(i, j);
    }

    // main rounds
    let extra_rounds = ((bytes[0].wrapping_add(bytes[bytes.len() - 1])) % 255) as usize % 1000;
    let num_rounds = 10_000 + extra_rounds;
    for _ in 0..num_rounds {
        for i in 0..state.len() {
            let j = state.len() - 1 - i;
            state[i] ^= state[j].rotate_left(7);
            state[i] = state[i].wrapping_mul(0x9E);
            state[i] = state[i].wrapping_add(state[(i + 1) % 8]);
            state[i] ^= state[(i + 3) % len].rotate_left(17);
            state[i] ^= block[(i + 4) % len].rotate_left(3);

            let idx = ((i % len) ^ (len.wrapping_mul(state[i % len] as usize))) % 64;
            let rot = (block[idx].wrapping_add(sbox[idx as usize]) % 8) as u32;
            state[i] = state[i].rotate_left(rot);
            state[i] ^= state[(i + 7) % state.len()].rotate_left(3);

            // s-box substitution
            state[i] = sbox[state[i] as usize];
        }

        let mut indices: Vec<usize> = (0..state.len()).collect();
        indices.reverse();
        for &i in &indices {
            state[i] ^= state[(i + 3) % state.len()];
        }

        let mut salt = bytes[0]
            .wrapping_add(
                bytes[bytes.len() - 1].wrapping_mul(sbox[(bytes.len() - 1) % 256 as usize]),
            )
            .rotate_left(3);
        for i in 0..state.len() {
            salt ^= salt.wrapping_mul(bytes[i.wrapping_add(bytes.len() - 1) % bytes.len()]);
            state[i] ^= salt.rotate_left((i % 8) as u32);
        }

        // final mix
        for i in 0..state.len() {
            state[i] ^= state[(i + 3) % state.len()];
            state[i] = state[i].wrapping_add(state[(i + 5) % state.len()]);
            state[i] = state[i].wrapping_mul(state[(i + 3) % state.len()]);
            state[i] = state[i].rotate_left(5);

            let idx = ((i % len) ^ (len.wrapping_mul(state[(i + 3) % len] as usize))) % 64;
            let rot = (block[idx] % 8) as u32 + 1;
            state[i] = state[i].rotate_left(rot);
            state[i] ^= state[(i + 4) % state.len()].rotate_left(4);
        }
    }

    // end mixing
    for (s, b) in state.iter_mut().zip(block.iter()) {
        *s ^= *b;
        *s = s.rotate_left(sbox[(*s & 0xff) as usize] as u32);
        *s = s.wrapping_add(*b).wrapping_mul(3) ^ *b;
    }

    // print as hex value
    let hex_state: String = state.iter().map(|b| format!("{:02x}", b)).collect();
    println!("Zinc-LHA Hash: {}", hex_state);
}
