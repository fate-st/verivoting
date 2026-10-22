//! H0 transcript-byte binding and the isolated H1 SHA-512 R1CS gate.

use ark_ed_on_bls12_381::Fq;
use ark_r1cs_std::{
    alloc::AllocVar, boolean::Boolean, eq::EqGadget, prelude::ToBitsGadget, uint64::UInt64,
    uint8::UInt8,
};
use ark_relations::r1cs::{ConstraintSystem, SynthesisError};

const SHA512_IV: [u64; 8] = [
    0x6a09_e667_f3bc_c908,
    0xbb67_ae85_84ca_a73b,
    0x3c6e_f372_fe94_f82b,
    0xa54f_f53a_5f1d_36f1,
    0x510e_527f_ade6_82d1,
    0x9b05_688c_2b3e_6c1f,
    0x1f83_d9ab_fb41_bd6b,
    0x5be0_cd19_137e_2179,
];

const SHA512_K: [u64; 80] = [
    0x428a_2f98_d728_ae22,
    0x7137_4491_23ef_65cd,
    0xb5c0_fbcf_ec4d_3b2f,
    0xe9b5_dba5_8189_dbbc,
    0x3956_c25b_f348_b538,
    0x59f1_11f1_b605_d019,
    0x923f_82a4_af19_4f9b,
    0xab1c_5ed5_da6d_8118,
    0xd807_aa98_a303_0242,
    0x1283_5b01_4570_6fbe,
    0x2431_85be_4ee4_b28c,
    0x550c_7dc3_d5ff_b4e2,
    0x72be_5d74_f27b_896f,
    0x80de_b1fe_3b16_96b1,
    0x9bdc_06a7_25c7_1235,
    0xc19b_f174_cf69_2694,
    0xe49b_69c1_9ef1_4ad2,
    0xefbe_4786_384f_25e3,
    0x0fc1_9dc6_8b8c_d5b5,
    0x240c_a1cc_77ac_9c65,
    0x2de9_2c6f_592b_0275,
    0x4a74_84aa_6ea6_e483,
    0x5cb0_a9dc_bd41_fbd4,
    0x76f9_88da_8311_53b5,
    0x983e_5152_ee66_dfab,
    0xa831_c66d_2db4_3210,
    0xb003_27c8_98fb_213f,
    0xbf59_7fc7_beef_0ee4,
    0xc6e0_0bf3_3da8_8fc2,
    0xd5a7_9147_930a_a725,
    0x06ca_6351_e003_826f,
    0x1429_2967_0a0e_6e70,
    0x27b7_0a85_46d2_2ffc,
    0x2e1b_2138_5c26_c926,
    0x4d2c_6dfc_5ac4_2aed,
    0x5338_0d13_9d95_b3df,
    0x650a_7354_8baf_63de,
    0x766a_0abb_3c77_b2a8,
    0x81c2_c92e_47ed_aee6,
    0x9272_2c85_1482_353b,
    0xa2bf_e8a1_4cf1_0364,
    0xa81a_664b_bc42_3001,
    0xc24b_8b70_d0f8_9791,
    0xc76c_51a3_0654_be30,
    0xd192_e819_d6ef_5218,
    0xd699_0624_5565_a910,
    0xf40e_3585_5771_202a,
    0x106a_a070_32bb_d1b8,
    0x19a4_c116_b8d2_d0c8,
    0x1e37_6c08_5141_ab53,
    0x2748_774c_df8e_eb99,
    0x34b0_bcb5_e19b_48a8,
    0x391c_0cb3_c5c9_5a63,
    0x4ed8_aa4a_e341_8acb,
    0x5b9c_ca4f_7763_e373,
    0x682e_6ff3_d6b2_b8a3,
    0x748f_82ee_5def_b2fc,
    0x78a5_636f_4317_2f60,
    0x84c8_7814_a1f0_ab72,
    0x8cc7_0208_1a64_39ec,
    0x90be_fffa_2363_1e28,
    0xa450_6ceb_de82_bde9,
    0xbef9_a3f7_b2c6_7915,
    0xc671_78f2_e372_532b,
    0xca27_3ece_ea26_619c,
    0xd186_b8c7_21c0_c207,
    0xeada_7dd6_cde0_eb1e,
    0xf57d_4f7f_ee6e_d178,
    0x06f0_67aa_7217_6fba,
    0x0a63_7dc5_a2c8_98a6,
    0x113f_9804_bef9_0dae,
    0x1b71_0b35_131c_471b,
    0x28db_77f5_2304_7d84,
    0x32ca_ab7b_40c7_2493,
    0x3c9e_be0a_15c9_bebc,
    0x431d_67c4_9c10_0d4c,
    0x4cc5_d4be_cb3e_42b6,
    0x597f_299c_fc65_7e2a,
    0x5fcb_6fab_3ad6_faec,
    0x6c44_198c_4a47_5817,
];

#[derive(Clone, Debug)]
pub struct TranscriptBindingReport {
    pub satisfied: bool,
    pub constraints: usize,
    pub public_inputs: usize,
    pub witness_variables: usize,
    pub byte_length: usize,
}

#[derive(Clone, Debug)]
pub struct Sha512CircuitReport {
    pub satisfied: bool,
    pub constraints: usize,
    pub public_inputs: usize,
    pub witness_variables: usize,
    pub byte_length: usize,
    pub padded_blocks: usize,
}

fn xor3(a: &UInt64<Fq>, b: &UInt64<Fq>, c: &UInt64<Fq>) -> Result<UInt64<Fq>, SynthesisError> {
    a.xor(b)?.xor(c)
}

fn bitand(a: &UInt64<Fq>, b: &UInt64<Fq>) -> Result<UInt64<Fq>, SynthesisError> {
    let a_bits = a.to_bits_le();
    let b_bits = b.to_bits_le();
    let bits = a_bits
        .iter()
        .zip(&b_bits)
        .map(|(left, right)| left.and(right))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(UInt64::from_bits_le(&bits))
}

fn shr(value: &UInt64<Fq>, by: usize) -> Result<UInt64<Fq>, SynthesisError> {
    let bits = value.to_bits_le();
    let shifted = bits
        .iter()
        .skip(by)
        .cloned()
        .chain(std::iter::repeat_n(Boolean::FALSE, by))
        .collect::<Vec<_>>();
    Ok(UInt64::from_bits_le(&shifted))
}

fn add_words(words: &[UInt64<Fq>]) -> Result<UInt64<Fq>, SynthesisError> {
    let mut sum = UInt64::constant(0);
    for word in words {
        sum = UInt64::addmany(&[sum, word.clone()])?;
    }
    Ok(sum)
}

fn choose(x: &UInt64<Fq>, y: &UInt64<Fq>, z: &UInt64<Fq>) -> Result<UInt64<Fq>, SynthesisError> {
    let y_xor_z = y.xor(z)?;
    z.xor(&bitand(x, &y_xor_z)?)
}

fn majority(x: &UInt64<Fq>, y: &UInt64<Fq>, z: &UInt64<Fq>) -> Result<UInt64<Fq>, SynthesisError> {
    let x_xor_y = x.xor(y)?;
    let x_xor_z = x.xor(z)?;
    x.xor(&bitand(&x_xor_y, &x_xor_z)?)
}

fn big_sigma0(x: &UInt64<Fq>) -> Result<UInt64<Fq>, SynthesisError> {
    xor3(&x.rotr(28), &x.rotr(34), &x.rotr(39))
}

fn big_sigma1(x: &UInt64<Fq>) -> Result<UInt64<Fq>, SynthesisError> {
    xor3(&x.rotr(14), &x.rotr(18), &x.rotr(41))
}

fn small_sigma0(x: &UInt64<Fq>) -> Result<UInt64<Fq>, SynthesisError> {
    xor3(&x.rotr(1), &x.rotr(8), &shr(x, 7)?)
}

fn small_sigma1(x: &UInt64<Fq>) -> Result<UInt64<Fq>, SynthesisError> {
    xor3(&x.rotr(19), &x.rotr(61), &shr(x, 6)?)
}

fn word_from_be_bytes(bytes: &[UInt8<Fq>]) -> Result<UInt64<Fq>, SynthesisError> {
    debug_assert_eq!(bytes.len(), 8);
    let mut bits = Vec::with_capacity(64);
    for byte in bytes.iter().rev() {
        bits.extend(byte.to_bits_le()?);
    }
    Ok(UInt64::from_bits_le(&bits))
}

fn word_to_be_bytes(word: &UInt64<Fq>) -> Result<Vec<UInt8<Fq>>, SynthesisError> {
    let bits = word.to_bits_le();
    let mut bytes = bits.chunks(8).map(UInt8::from_bits_le).collect::<Vec<_>>();
    bytes.reverse();
    Ok(bytes)
}

fn pad_sha512(input: &[UInt8<Fq>]) -> Vec<UInt8<Fq>> {
    let bit_length = (input.len() as u128) * 8;
    let mut padded = input.to_vec();
    padded.push(UInt8::constant(0x80));
    while padded.len() % 128 != 112 {
        padded.push(UInt8::constant(0));
    }
    padded.extend(UInt8::constant_vec(&bit_length.to_be_bytes()));
    padded
}

fn sha512_digest_var(input: &[UInt8<Fq>]) -> Result<Vec<UInt8<Fq>>, SynthesisError> {
    let padded = pad_sha512(input);
    let mut state = SHA512_IV.map(UInt64::constant);

    let (blocks, block_remainder) = padded.as_chunks::<128>();
    debug_assert!(block_remainder.is_empty());
    for block in blocks {
        let mut schedule = Vec::with_capacity(80);
        let (words, word_remainder) = block.as_slice().as_chunks::<8>();
        debug_assert!(word_remainder.is_empty());
        for bytes in words {
            schedule.push(word_from_be_bytes(bytes)?);
        }
        for index in 16..80 {
            schedule.push(add_words(&[
                small_sigma1(&schedule[index - 2])?,
                schedule[index - 7].clone(),
                small_sigma0(&schedule[index - 15])?,
                schedule[index - 16].clone(),
            ])?);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state.clone();
        for round in 0..80 {
            let temp1 = add_words(&[
                h.clone(),
                big_sigma1(&e)?,
                choose(&e, &f, &g)?,
                UInt64::constant(SHA512_K[round]),
                schedule[round].clone(),
            ])?;
            let temp2 = add_words(&[big_sigma0(&a)?, majority(&a, &b, &c)?])?;
            h = g;
            g = f;
            f = e;
            e = add_words(&[d, temp1.clone()])?;
            d = c;
            c = b;
            b = a;
            a = add_words(&[temp1, temp2])?;
        }
        state = [
            add_words(&[state[0].clone(), a])?,
            add_words(&[state[1].clone(), b])?,
            add_words(&[state[2].clone(), c])?,
            add_words(&[state[3].clone(), d])?,
            add_words(&[state[4].clone(), e])?,
            add_words(&[state[5].clone(), f])?,
            add_words(&[state[6].clone(), g])?,
            add_words(&[state[7].clone(), h])?,
        ];
    }

    let mut digest = Vec::with_capacity(64);
    for word in &state {
        digest.extend(word_to_be_bytes(word)?);
    }
    Ok(digest)
}

pub fn check_sha512_digest(
    transcript: &[u8],
    expected_digest: &[u8],
) -> Result<Sha512CircuitReport, SynthesisError> {
    if expected_digest.len() != 64 {
        return Ok(Sha512CircuitReport {
            satisfied: false,
            constraints: 0,
            public_inputs: 0,
            witness_variables: 0,
            byte_length: transcript.len(),
            padded_blocks: 0,
        });
    }
    let cs = ConstraintSystem::<Fq>::new_ref();
    let transcript_var = UInt8::new_witness_vec(cs.clone(), transcript)?;
    let expected_var = UInt8::new_input_vec(cs.clone(), expected_digest)?;
    let digest_var = sha512_digest_var(&transcript_var)?;
    digest_var.enforce_equal(&expected_var)?;
    let satisfied = cs.is_satisfied()?;
    Ok(Sha512CircuitReport {
        satisfied,
        constraints: cs.num_constraints(),
        public_inputs: cs.num_instance_variables().saturating_sub(1),
        witness_variables: cs.num_witness_variables(),
        byte_length: transcript.len(),
        padded_blocks: (transcript.len() + 1 + 16).div_ceil(128),
    })
}

pub fn check_transcript_byte_binding(
    witness_bytes: &[u8],
    public_bytes: &[u8],
) -> Result<TranscriptBindingReport, SynthesisError> {
    if witness_bytes.len() != public_bytes.len() {
        return Ok(TranscriptBindingReport {
            satisfied: false,
            constraints: 0,
            public_inputs: 0,
            witness_variables: 0,
            byte_length: witness_bytes.len(),
        });
    }
    let cs = ConstraintSystem::<Fq>::new_ref();
    for (witness, public) in witness_bytes.iter().zip(public_bytes) {
        let witness_var = UInt8::new_witness(cs.clone(), || Ok(*witness))?;
        let public_var = UInt8::new_input(cs.clone(), || Ok(*public))?;
        witness_var.enforce_equal(&public_var)?;
    }
    let satisfied = cs.is_satisfied()?;
    Ok(TranscriptBindingReport {
        satisfied,
        constraints: cs.num_constraints(),
        public_inputs: cs.num_instance_variables().saturating_sub(1),
        witness_variables: cs.num_witness_variables(),
        byte_length: witness_bytes.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha512};

    #[test]
    fn exact_bytes_satisfy_and_changed_or_reordered_bytes_fail() {
        let transcript = b"canonical-cp-transcript-v3";
        assert!(
            check_transcript_byte_binding(transcript, transcript)
                .unwrap()
                .satisfied
        );

        let mut changed = transcript.to_vec();
        changed[0] ^= 1;
        assert!(
            !check_transcript_byte_binding(transcript, &changed)
                .unwrap()
                .satisfied
        );

        let mut reordered = transcript.to_vec();
        reordered.swap(1, 2);
        assert!(
            !check_transcript_byte_binding(transcript, &reordered)
                .unwrap()
                .satisfied
        );
    }

    #[test]
    fn sha512_matches_native_and_rejects_changed_digest() {
        let message = b"abc";
        let digest = Sha512::digest(message);
        assert!(check_sha512_digest(message, &digest).unwrap().satisfied);

        let mut changed = digest.to_vec();
        changed[0] ^= 1;
        assert!(!check_sha512_digest(message, &changed).unwrap().satisfied);
    }
}
