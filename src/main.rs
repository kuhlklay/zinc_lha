use std::io::{stdin, stdout, Write};

/// --- Helper Functions ---

/// Performs a left rotation on a byte by a given number of bits.
///
/// # Parameters
/// - `val`: The byte to rotate.
/// - `bits`: Number of bits to rotate left.
///
/// # Returns
/// The rotated byte.
fn rotl(val: u8, bits: u32) -> u8 {
    val.rotate_left(bits)
}

/// Performs a lookup in a substitution box (S-box).
///
/// # Parameters
/// - `sbox`: Reference to a 256-element array representing the S-box.
/// - `val`: The input byte to transform via the S-box.
///
/// # Returns
/// The transformed byte from the S-box.
fn sbox_lookup(sbox: &[u8; 256], val: u8) -> u8 {
    sbox[val as usize]
}

/// --- S-Box Initialization ---

/// Initializes a 256-element S-box using a key-dependent algorithm.
///
/// The initialization uses a two-pass mixing process:
/// 1. First pass: mixes S-box entries with input bytes and pseudo-random rotations.
/// 2. Second pass: further scrambles the S-box using previous S-box values.
///
/// # Parameters
/// - `bytes`: Key/input data to seed the S-box.
///
/// # Returns
/// A fully initialized 256-byte S-box.
fn init_sbox(bytes: &[u8]) -> [u8; 256] {
    let len = bytes.len();
    let mut sbox = [0u8; 256];

    // Initialize S-box sequentially
    for i in 0..256 {
        sbox[i] = i as u8;
    }

    let mut seed = bytes[0].wrapping_add(bytes[len - 1]);

    // --- First Mixing Pass ---
    for i in (1..256).rev() {
        seed = seed.wrapping_add(bytes[i % len])
            ^ sbox[i]
            ^ sbox[(i + 7) % 256]
            ^ seed.rotate_left((i % 5) as u32);
        let j = (seed as usize ^ i) % 256;
        sbox[i] ^= bytes[i % len];
        sbox[j] ^= bytes[(i + 1) % len];
        sbox.swap(i, j);
    }

    // --- Second Mixing Pass ---
    for i in (1..256).rev() {
        seed ^= sbox[(i * 3) % 256];
        let j = (seed as usize ^ i) % 256;
        sbox[i] ^= bytes[(i + 16) % len];
        sbox[j] ^= bytes[(i + (i.wrapping_mul(11) ^ i)) % len];
        sbox.swap(i, j);
    }

    sbox
}

/// --- Core Hash Round Function ---

/// Performs a single round of the Zinc-LHA hash function on a 64-byte state.
///
/// Each byte in the state undergoes multiple transformations:
/// - XOR with rotated state bytes
/// - Multiplication and addition
/// - XOR with rotated input block bytes
/// - Rotations based on S-box and block values
/// - Substitution via S-box lookup
/// - Additional mixing and "salt" injection based on input bytes
///
/// # Parameters
/// - `state`: Mutable reference to the 64-byte hash state.
/// - `block`: Reference to the current 64-byte input block.
/// - `sbox`: Reference to the initialized 256-byte S-box.
/// - `bytes`: Original input key/bytes for salt mixing.
fn round(state: &mut [u8; 64], block: &[u8; 64], sbox: &[u8; 256], bytes: &[u8]) {
    let len = bytes.len().min(64);

    // Main byte-wise transformations
    for i in 0..state.len() {
        let j = state.len() - 1 - i;
        state[i] ^= rotl(state[j], 7);
        state[i] = state[i].wrapping_mul(0x9E).wrapping_add(state[(i + 1) % 8]);
        state[i] ^= rotl(block[(i + 4) % len], 3);

        let idx = ((i % len) ^ (len.wrapping_mul(state[i % len] as usize))) % 64;
        state[i] = rotl(state[i], (block[idx].wrapping_add(sbox[idx]) % 8) as u32);
        state[i] ^= rotl(state[(i + 7) % state.len()], 3);
        state[i] = sbox_lookup(sbox, state[i]);
    }

    // Simple cross-byte mixing
    for i in 0..state.len() {
        state[i] ^= state[(i + 3) % state.len()];
    }

    // Salt-based mixing from input key
    let mut salt = bytes[0]
        .wrapping_add(bytes[bytes.len() - 1].wrapping_mul(sbox[(bytes.len() - 1) % 256]))
        .rotate_left(3);

    for i in 0..state.len() {
        salt ^= salt.wrapping_mul(bytes[i.wrapping_add(bytes.len() - 1) % bytes.len()]);
        state[i] ^= rotl(salt, (i % 8) as u32);
    }

    // Final intra-round mixing
    for i in 0..state.len() {
        state[i] ^= state[(i + 3) % state.len()];
        state[i] = state[i]
            .wrapping_add(state[(i + 5) % state.len()])
            .wrapping_mul(state[(i + 3) % state.len()])
            .rotate_left(5);

        let idx = ((i % len) ^ (len.wrapping_mul(state[(i + 3) % len] as usize))) % 64;
        state[i] = rotl(state[i], (block[idx] % 8) as u32 + 1);
        state[i] ^= rotl(state[(i + 4) % state.len()], 4);
    }
}

/// --- End-of-Hash Mixing ---

/// Performs a final mixing of the hash state with the input block and S-box.
///
/// This ensures that each byte of the state is influenced by the input block and
/// S-box in a non-linear manner, improving diffusion.
///
/// # Parameters
/// - `state`: Mutable reference to the 64-byte hash state.
/// - `block`: Reference to the input block.
/// - `sbox`: Reference to the initialized S-box.
fn end_mix(state: &mut [u8; 64], block: &[u8; 64], sbox: &[u8; 256]) {
    for (s, b) in state.iter_mut().zip(block.iter()) {
        *s ^= *b;
        *s = rotl(*s, sbox[*s as usize] as u32);
        *s = s.wrapping_add(*b).wrapping_mul(3) ^ *b;
    }
}

/// --- Main Entry Point ---

fn main() {
    // Read input from the user
    let mut input = String::new();
    print!("Input: ");
    let _ = stdout().flush();
    stdin().read_line(&mut input).expect("Error reading input");

    let mut bytes = input.trim().as_bytes().to_vec();
    if bytes.is_empty() {
        bytes.push(0x00); // Ensure non-empty input
    }

    // Initialize the 64-byte hash state
    let mut state = [0u8; 64];
    state[0] ^= bytes.len() as u8;
    state[1] ^= (bytes.len() >> 8) as u8;

    // Prepare the initial block for processing
    let mut block = [state[0]; 64];
    let len = bytes.len().min(64);
    block[..len].copy_from_slice(&bytes[..len]);

    // Initialize the S-box from the input bytes
    let sbox = init_sbox(&bytes);

    // Determine the number of rounds dynamically from input
    let extra_rounds = ((bytes[0].wrapping_add(bytes[bytes.len() - 1])) % 255) as usize % 1000;
    let num_rounds = 10_000 + extra_rounds;

    // Perform the main hash rounds
    for _ in 0..num_rounds {
        round(&mut state, &block, &sbox, &bytes);
    }

    // Apply the final mixing
    end_mix(&mut state, &block, &sbox);

    // Convert final hash state to hexadecimal string
    let hex_state: String = state.iter().map(|b| format!("{:02x}", b)).collect();
    println!("Zinc-LHA Hash: {}", hex_state);
}
