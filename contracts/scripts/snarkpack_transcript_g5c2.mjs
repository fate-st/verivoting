// G5-C2 wire/transcript conformance only. This is NOT a SnarkPack verifier:
// it checks encodings and derives Fiat--Shamir challenges, but does not check
// GIPA, KZG, aggregate pairing equations, or EVM execution.
import { createHash } from 'node:crypto';
import { bls12_381 } from '@noble/curves/bls12-381.js';

const { Fp, Fp6, Fp12, Fr } = bls12_381.fields;
const PREFIX = Buffer.from('snarkpack-v1', 'ascii');

function bytesFromHex(value, name) {
  if (typeof value !== 'string' || !/^(?:[0-9a-fA-F]{2})*$/.test(value)) {
    throw new Error(`${name}: expected even-length hex without a 0x prefix`);
  }
  return Buffer.from(value, 'hex');
}

function fromLE(bytes) {
  return BigInt(`0x${Buffer.from(bytes).reverse().toString('hex') || '0'}`);
}

function toLE(value, length) {
  if (value < 0n || value >= (1n << BigInt(8 * length))) {
    throw new Error('integer does not fit canonical byte length');
  }
  return Buffer.from(value.toString(16).padStart(length * 2, '0'), 'hex').reverse();
}

function fp6Coefficients(value) {
  return [value.c0.c0, value.c0.c1, value.c1.c0,
    value.c1.c1, value.c2.c0, value.c2.c1];
}

function serializeGT(value) {
  // blstrs serde_impl.rs: Fp12 -> Fp6 c0/c1 -> Fp2 c0/c1,
  // each Fp is six little-endian u64s, without any framing.
  return Buffer.concat([
    ...fp6Coefficients(value.c0), ...fp6Coefficients(value.c1),
  ].map((coefficient) => toLE(coefficient, 48)));
}

function decompressGT(wire) {
  const coefficients = Array.from({ length: 6 }, (_, i) => {
    const value = fromLE(wire.subarray(i * 48, (i + 1) * 48));
    if (value >= Fp.ORDER) throw new Error('non-canonical compressed GT coefficient');
    return value;
  });
  const compressed = Fp6.fromBigSix(coefficients);
  // Rubin--Silverberg torus decompression, exactly the blstrs formula:
  // (compressed + w) / (compressed - w), w^2 = v.
  const numerator = Fp12.create({ c0: compressed, c1: Fp6.ONE });
  const denominator = Fp12.create({ c0: compressed, c1: Fp6.neg(Fp6.ONE) });
  const value = Fp12.mul(numerator, Fp12.inv(denominator));
  if (!Fp12.eql(Fp12.pow(value, Fr.ORDER), Fp12.ONE)) {
    throw new Error('compressed GT does not decode into the prime-order subgroup');
  }
  // Check that decompression preserves the canonical wire representative.
  const recompressed = Fp6.mul(Fp6.add(value.c0, Fp6.ONE), Fp6.inv(value.c1));
  const roundtrip = Buffer.concat(fp6Coefficients(recompressed).map((c) => toLE(c, 48)));
  if (!roundtrip.equals(wire)) throw new Error('GT compression roundtrip mismatch');
  return serializeGT(value);
}

function readProof(proofHex) {
  const wire = bytesFromHex(proofHex, 'aggregate proof');
  if (wire.length !== 5236) throw new Error('G5-C2 requires the exact 5236-byte B=2 wire proof');
  let offset = 0;
  const take = (length) => {
    if (offset + length > wire.length) throw new Error('truncated aggregate proof');
    const bytes = wire.subarray(offset, offset + length);
    offset += length;
    return bytes;
  };
  const gt = () => decompressGT(take(288));
  const point = (group, length) => {
    const bytes = take(length);
    const value = group.Point.fromBytes(bytes); // includes curve/subgroup validation
    if (!Buffer.from(value.toBytes(true)).equals(bytes)) {
      throw new Error('non-canonical compressed group point');
    }
    return bytes;
  };
  const g1 = () => point(bls12_381.G1, 48);
  const g2 = () => point(bls12_381.G2, 96);
  const pair = (read) => [read(), read()];
  const proof = {
    comAB: pair(gt),
    comC: pair(gt),
    ipAB: gt(),
    aggC: g1(),
  };
  if (take(4).readUInt32LE() !== 2) throw new Error('only B=2 is supported by this gate');
  proof.commsAB = [pair(gt), pair(gt)];
  proof.commsC = [pair(gt), pair(gt)];
  proof.zAB = pair(gt);
  proof.zC = pair(g1);
  proof.finalA = g1();
  proof.finalB = g2();
  proof.finalC = g1();
  proof.finalVkey = pair(g2);
  proof.finalWkey = pair(g1);
  proof.vkeyOpening = pair(g2);
  proof.wkeyOpening = pair(g1);
  if (offset !== wire.length) throw new Error('trailing aggregate proof data');
  return proof;
}

function challenge(id, tag, fields) {
  const payload = Buffer.concat(fields);
  const base = Buffer.concat([PREFIX, Buffer.from(tag, 'ascii'), payload]);
  const hash = createHash('sha256').update(base);
  const attempts = [];
  // bellperson uses usize.to_be_bytes(). This gate fixes the measured 64-bit
  // platform's width, not a new architecture-independent SnarkPack suite.
  for (let counter = 1; ; counter += 1) {
    const nonce = Buffer.alloc(8);
    nonce.writeBigUInt64BE(BigInt(counter));
    hash.update(nonce); // rejected counters remain in the hash state
    const digest = hash.copy().digest();
    const candidate = Buffer.from(digest);
    candidate[31] &= 0x7f;
    const scalar = fromLE(candidate);
    const accepted = scalar > 1n && scalar < Fr.ORDER;
    attempts.push({
      counter,
      digest_hex: digest.toString('hex'),
      candidate_le_hex: candidate.toString('hex'),
      accepted,
    });
    if (accepted) {
      return {
        id, tag, base_hex: base.toString('hex'), payload_bytes: payload.length,
        attempts, challenge_le_hex: candidate.toString('hex'),
      };
    }
    if (!Number.isSafeInteger(counter + 1)) throw new Error('challenge counter overflow');
  }
}

/**
 * Independently decode B=2 aggregate proof wire and reproduce the six internal
 * bellperson 0.27.0 / AggregateVersion::V2 challenge calls. Never accepts Rust
 * preimages or GT coefficients; the only inputs are wire proof + external bytes.
 * Does not verify the complete SnarkPack relation or any EVM verifier.
 */
export function traceSnarkpackB2(proofHex, externalHex) {
  const proof = readProof(proofHex);
  const external = bytesFromHex(externalHex, 'external transcript');
  const traces = [];
  const derive = (id, tag, fields) => {
    const trace = challenge(id, tag, fields);
    traces.push(trace);
    return Buffer.from(trace.challenge_le_hex, 'hex');
  };
  const hcom = derive('hcom', 'hcom', [...proof.comAB, ...proof.comC]);
  const r = derive('random-r', 'random-r', [hcom, toLE(BigInt(external.length), 8), external]);
  const seed = derive('gipa-0-seed', 'gipa-0', [hcom, proof.ipAB, proof.aggC, r]);
  const roundFields = [
    ...proof.zAB, ...proof.zC,
    ...proof.commsAB[0], ...proof.commsAB[1],
    ...proof.commsC[0], ...proof.commsC[1],
  ];
  const roundInverse = derive('gipa-0-round', 'gipa-0', [seed, ...roundFields]);
  // verifier stores challenges = inverse(c_inv); extra-link uses challenges.last().
  const roundChallenge = toLE(Fr.inv(fromLE(roundInverse)), 32);
  const extra = derive('extra-link', 'gipa-extra-link', [
    roundChallenge, proof.finalA, proof.finalB, proof.finalC, ...roundFields,
  ]);
  derive('random-z', 'random-z', [
    extra, ...proof.finalVkey, ...proof.finalWkey,
    proof.finalA, proof.finalB, proof.finalC,
  ]);
  return traces;
}
