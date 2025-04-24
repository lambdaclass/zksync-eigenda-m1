use ark_bn254::{G1Affine, G2Affine};
use ark_ff::{Fp, Fp2};
use ark_serialize::CanonicalSerialize;
use serde::ser::Error;
use crate::verify_blob::{BlobCommitment as BlobCommitmentsContract, BlobHeaderV2 as BlobHeaderV2Contract, BlobCertificate as BlobCertificateContract, BlobInclusionInfo as BlobInclusionInfoContract, BatchHeaderV2 as BatchHeaderV2Contract, NonSignerStakesAndSignature as NonSignerStakesAndSignatureContract, Attestation as AttestationContract};
use rust_kzg_bn254_primitives::helpers::{lexicographically_largest, read_g1_point_from_bytes_be};
use ark_ff::Zero;

#[derive(Debug, PartialEq, Clone)]
/// BlomCommitments contains the blob's commitment, degree proof, and the actual degree.
pub struct BlobCommitments {
    pub commitment: G1Affine,
    pub length_commitment: G2Affine,
    pub length_proof: G2Affine,
    pub length: u32,
}

/// Helper struct for BlobCommitments,
/// for simpler serialization, and deserialization
#[derive(serde::Serialize, serde::Deserialize)]
struct BlobCommitmentsHelper {
    commitment: Vec<u8>,
    length_commitment: Vec<u8>,
    length_proof: Vec<u8>,
    length: u32,
}

impl TryFrom<&BlobCommitments> for BlobCommitmentsHelper {
    type Error = anyhow::Error;

    fn try_from(b: &BlobCommitments) -> Result<Self, Self::Error> {
        Ok(BlobCommitmentsHelper {
            commitment: g1_commitment_to_bytes(&b.commitment)?,
            length_commitment: g2_commitment_to_bytes(&b.length_commitment)?,
            length_proof: g2_commitment_to_bytes(&b.length_proof)?,
            length: b.length,
        })
    }
}

impl TryFrom<BlobCommitmentsHelper> for BlobCommitments {
    type Error = anyhow::Error;

    fn try_from(helper: BlobCommitmentsHelper) -> Result<Self, Self::Error> {
        Ok(BlobCommitments {
            commitment: g1_commitment_from_bytes(&helper.commitment)?,
            length_commitment: g2_commitment_from_bytes(&helper.length_commitment)?,
            length_proof: g2_commitment_from_bytes(&helper.length_proof)?,
            length: helper.length,
        })
    }
}

impl serde::Serialize for BlobCommitments {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        BlobCommitmentsHelper::try_from(self)
            .map_err(|e| S::Error::custom(format!("Conversion failed: {}", e)))?
            .serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for BlobCommitments {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = BlobCommitmentsHelper::deserialize(deserializer)?;
        Self::try_from(helper).map_err(serde::de::Error::custom)
    }
}

impl From<BlobCommitments> for BlobCommitmentsContract {
    fn from(value: BlobCommitments) -> Self {
        Self {
            lengthCommitment: g2_contract_point_from_g2_affine(&value.length_commitment),
            lengthProof: g2_contract_point_from_g2_affine(&value.length_proof),
            length: value.length,
            commitment: g1_contract_point_from_g1_affine(&value.commitment),
        }
    }
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlobHeader {
    pub(crate) version: u16,
    pub(crate) quorum_numbers: Vec<u8>,
    pub(crate) commitment: BlobCommitments,
    pub(crate) payment_header_hash: [u8; 32],
}

impl From<BlobHeader> for BlobHeaderV2Contract {
    fn from(value: BlobHeader) -> Self {
        Self {
            version: value.version,
            quorumNumbers: value.quorum_numbers.clone().into(),
            commitment: value.commitment.clone().into(),
            paymentHeaderHash: value.payment_header_hash,
        }
    }
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
/// BlobCertificate contains a full description of a blob and how it is dispersed. Part of the certificate
/// is provided by the blob submitter (i.e. the blob header), and part is provided by the disperser (i.e. the relays).
/// Validator nodes eventually sign the blob certificate once they are in custody of the required chunks
/// (note that the signature is indirect; validators sign the hash of a Batch, which contains the blob certificate).
pub struct BlobCertificate {
    pub blob_header: BlobHeader,
    pub signature: Vec<u8>,
    pub relay_keys: Vec<u32>,
}

impl From<BlobCertificate> for BlobCertificateContract {
    fn from(value: BlobCertificate) -> Self {
        Self {
            blobHeader: value.blob_header.into(),
            signature: value.signature.into(),
            relayKeys: value.relay_keys,
        }
    }
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
/// BlobInclusionInfo is the information needed to verify the inclusion of a blob in a batch.
pub struct BlobInclusionInfo {
    pub blob_certificate: BlobCertificate,
    pub blob_index: u32,
    pub inclusion_proof: Vec<u8>,
}

impl From<BlobInclusionInfo> for BlobInclusionInfoContract {
    fn from(value: BlobInclusionInfo) -> Self {
        BlobInclusionInfoContract {
            blobCertificate: value.blob_certificate.into(),
            blobIndex: value.blob_index,
            inclusionProof: value.inclusion_proof.clone().into(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchHeaderV2 {
    pub batch_root: [u8; 32],
    pub reference_block_number: u32,
}

impl From<BatchHeaderV2> for BatchHeaderV2Contract {
    fn from(value: BatchHeaderV2) -> Self {
        Self {
            batchRoot: value.batch_root,
            referenceBlockNumber: value.reference_block_number,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct NonSignerStakesAndSignature {
    pub non_signer_quorum_bitmap_indices: Vec<u32>,
    pub non_signer_pubkeys: Vec<G1Affine>,
    pub quorum_apks: Vec<G1Affine>,
    pub apk_g2: G2Affine,
    pub sigma: G1Affine,
    pub quorum_apk_indices: Vec<u32>,
    pub total_stake_indices: Vec<u32>,
    pub non_signer_stake_indices: Vec<Vec<u32>>,
}

impl From<NonSignerStakesAndSignature> for NonSignerStakesAndSignatureContract {
    fn from(value: NonSignerStakesAndSignature) -> Self {
        Self {
            nonSignerQuorumBitmapIndices: value.non_signer_quorum_bitmap_indices.clone(),
            nonSignerPubkeys: value
                .non_signer_pubkeys
                .iter()
                .map(g1_contract_point_from_g1_affine)
                .collect(),
                quorumApks: value
                .quorum_apks
                .iter()
                .map(g1_contract_point_from_g1_affine)
                .collect(),
                apkG2: g2_contract_point_from_g2_affine(&value.apk_g2),
            sigma: g1_contract_point_from_g1_affine(&value.sigma),
            quorumApkIndices: value.quorum_apk_indices.clone(),
            totalStakeIndices: value.total_stake_indices.clone(),
            nonSignerStakeIndices: value.non_signer_stake_indices.clone(),
        }
    }
}

/// Helper struct for serialization and deserialization of NonSignerStakesAndSignature
#[derive(serde::Serialize, serde::Deserialize)]
struct NonSignerStakesAndSignatureHelper {
    non_signer_quorum_bitmap_indices: Vec<u32>,
    non_signer_pubkeys: Vec<Vec<u8>>,
    quorum_apks: Vec<Vec<u8>>,
    apk_g2: Vec<u8>,
    sigma: Vec<u8>,
    quorum_apk_indices: Vec<u32>,
    total_stake_indices: Vec<u32>,
    non_signer_stake_indices: Vec<Vec<u32>>,
}

impl TryFrom<&NonSignerStakesAndSignature> for NonSignerStakesAndSignatureHelper {
    type Error = anyhow::Error;

    fn try_from(n: &NonSignerStakesAndSignature) -> Result<Self, Self::Error> {
        Ok(NonSignerStakesAndSignatureHelper {
            non_signer_quorum_bitmap_indices: n.non_signer_quorum_bitmap_indices.clone(),
            non_signer_pubkeys: n
                .non_signer_pubkeys
                .iter()
                .map(g1_commitment_to_bytes)
                .collect::<Result<_, _>>()?,
            quorum_apks: n
                .quorum_apks
                .iter()
                .map(g1_commitment_to_bytes)
                .collect::<Result<_, _>>()?,
            apk_g2: g2_commitment_to_bytes(&n.apk_g2)?,
            sigma: g1_commitment_to_bytes(&n.sigma)?,
            quorum_apk_indices: n.quorum_apk_indices.clone(),
            total_stake_indices: n.total_stake_indices.clone(),
            non_signer_stake_indices: n.non_signer_stake_indices.clone(),
        })
    }
}

impl TryFrom<NonSignerStakesAndSignatureHelper> for NonSignerStakesAndSignature {
    type Error = anyhow::Error;

    fn try_from(helper: NonSignerStakesAndSignatureHelper) -> Result<Self, Self::Error> {
        Ok(NonSignerStakesAndSignature {
            non_signer_quorum_bitmap_indices: helper.non_signer_quorum_bitmap_indices,
            non_signer_pubkeys: helper
                .non_signer_pubkeys
                .iter()
                .map(|b| g1_commitment_from_bytes(b))
                .collect::<Result<_, _>>()?,
            quorum_apks: helper
                .quorum_apks
                .iter()
                .map(|b| g1_commitment_from_bytes(b))
                .collect::<Result<_, _>>()?,
            apk_g2: g2_commitment_from_bytes(&helper.apk_g2)?,
            sigma: g1_commitment_from_bytes(&helper.sigma)?,
            quorum_apk_indices: helper.quorum_apk_indices,
            total_stake_indices: helper.total_stake_indices,
            non_signer_stake_indices: helper.non_signer_stake_indices,
        })
    }
}

impl serde::Serialize for NonSignerStakesAndSignature {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        NonSignerStakesAndSignatureHelper::try_from(self)
            .map_err(|e| S::Error::custom(format!("Conversion failed: {}", e)))?
            .serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for NonSignerStakesAndSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = NonSignerStakesAndSignatureHelper::deserialize(deserializer)?;
        Self::try_from(helper).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Attestation {
    pub non_signer_pubkeys: Vec<G1Affine>,
    pub quorum_apks: Vec<G1Affine>,
    pub sigma: G1Affine,
    pub apk_g2: G2Affine,
    pub quorum_numbers: Vec<u32>,
}

impl From<Attestation> for AttestationContract {
    fn from(value: Attestation) -> Self {
        Self {
            non_signer_pubkeys: value
                .non_signer_pubkeys
                .iter()
                .map(g1_contract_point_from_g1_affine)
                .collect::<Vec<_>>(),
            quorum_apks: value
                .quorum_apks
                .iter()
                .map(g1_contract_point_from_g1_affine)
                .collect::<Vec<_>>(),
            sigma: g1_contract_point_from_g1_affine(&value.sigma),
            apk_g2: g2_contract_point_from_g2_affine(&value.apk_g2),
            quorum_numbers: value.quorum_numbers,
        }
    }
}

// EigenDACert contains all data necessary to retrieve and validate a blob
//
// This struct represents the composition of a eigenDA blob certificate, as it would exist in a rollup inbox.
#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct EigenDACert {
    pub blob_inclusion_info: BlobInclusionInfo,
    pub batch_header: BatchHeaderV2,
    pub non_signer_stakes_and_signature: NonSignerStakesAndSignature,
    pub signed_quorum_numbers: Vec<u8>,
}

const COMPRESSED_SMALLEST: u8 = 0b10 << 6;
const COMPRESSED_LARGEST: u8 = 0b11 << 6;
const COMPRESSED_INFINITY: u8 = 0b01 << 6;
const G2_COMPRESSED_SIZE: usize = 64;


/// g1_commitment_from_bytes converts a byte slice to a G1Affine point.
/// The points received are in compressed form.
pub fn g1_commitment_from_bytes(bytes: &[u8]) -> Result<G1Affine, anyhow::Error> {
    read_g1_point_from_bytes_be(bytes).map_err(|e| anyhow::anyhow!("Failed to read G1 point: {}", e))
}


/// Serialize a G1Affine point applying necessary flags.
/// https://github.com/Consensys/gnark-crypto/blob/5fd6610ac2a1d1b10fae06c5e552550bf43f4d44/ecc/bn254/marshal.go#L790-L801
pub fn g1_commitment_to_bytes(point: &G1Affine) -> Result<Vec<u8>, anyhow::Error> {
    let mut bytes = vec![0u8; 32];

    // Infinity case
    if point.to_flags().is_infinity() {
        bytes[0] = COMPRESSED_INFINITY;
        return Ok(bytes);
    }

    // Get X bytes
    let mut x_bytes = Vec::new();
    point.x.serialize_compressed(&mut x_bytes)?;
    bytes.copy_from_slice(&x_bytes);
    bytes.reverse();

    // Determine most significant bits flag
    let mask = match lexicographically_largest(&point.y) {
        true => COMPRESSED_LARGEST,
        false => COMPRESSED_SMALLEST,
    };
    bytes[0] |= mask;

    Ok(bytes)
}


/// g2_commitment_from_bytes converts a byte slice to a G2Affine point.
pub fn g2_commitment_from_bytes(bytes: &[u8]) -> Result<G2Affine, anyhow::Error> {
    if bytes.len() != 64 {
        return Err(anyhow::anyhow!(
            "Invalid length for G2 Commitment".to_string(),
        ));
    }

    // Get mask from most significant bits
    let msb_mask = bytes[0] & (COMPRESSED_INFINITY | COMPRESSED_SMALLEST | COMPRESSED_LARGEST);

    if msb_mask == COMPRESSED_INFINITY {
        return Ok(G2Affine::identity());
    }

    // Remove most significant bits mask
    let mut bytes = bytes.to_vec();
    bytes[0] &= !(COMPRESSED_INFINITY | COMPRESSED_SMALLEST | COMPRESSED_LARGEST);

    // Extract X from the compressed representation
    let x1 = Fp::from_be_bytes_mod_order(&bytes[0..32]);
    let x0 = Fp::from_be_bytes_mod_order(&bytes[32..64]);
    let x = Fp2::new(x0, x1);

    let mut point = G2Affine::get_point_from_x_unchecked(x, true).ok_or(
        anyhow::anyhow!("Failed to read G2 Commitment from x bytes".to_string()),
    )?;

    // Ensure Y has the correct lexicographic property
    let mut lex_largest = lexicographically_largest(&point.y.c1);
    if !lex_largest && point.y.c1.is_zero() {
        lex_largest = lexicographically_largest(&point.y.c0);
    }
    if (msb_mask == COMPRESSED_LARGEST) != lex_largest {
        point.y.neg_in_place();
    }

    Ok(point)
}

/// Convert bytes from little-endian to big-endian and vice versa.
fn switch_endianess(bytes: &mut Vec<u8>) {
    // Remove leading zeroes
    let mut filtered_bytes: Vec<u8> = bytes.iter().copied().skip_while(|&x| x == 0).collect();

    filtered_bytes.reverse();

    while filtered_bytes.len() != G2_COMPRESSED_SIZE {
        filtered_bytes.push(0);
    }

    *bytes = filtered_bytes;
}

/// Serialize a G2Affine point applying necessary flags.
pub fn g2_commitment_to_bytes(point: &G2Affine) -> Result<Vec<u8>, anyhow::Error> {
    let mut bytes = vec![0u8; 64];
    if point.to_flags().is_infinity() {
        bytes[0] |= COMPRESSED_INFINITY;
        return Ok(bytes);
    }
    point.serialize_compressed(&mut bytes)?;
    switch_endianess(&mut bytes);

    let mut lex_largest = lexicographically_largest(&point.y.c1);
    if !lex_largest && point.y.c1.is_zero() {
        lex_largest = lexicographically_largest(&point.y.c0);
    }

    let mask = match lex_largest {
        true => COMPRESSED_LARGEST,
        false => COMPRESSED_SMALLEST,
    };

    bytes[0] |= mask;
    Ok(bytes)
}


fn g2_contract_point_from_g2_affine(g2_affine: &G2Affine) -> G2PointContract {
    let x = g2_affine.x;
    let y = g2_affine.y;
    G2PointContract {
        x: [
            U256::from_big_endian(&x.c1.into_bigint().to_bytes_be()),
            U256::from_big_endian(&x.c0.into_bigint().to_bytes_be()),
        ],
        y: [
            U256::from_big_endian(&y.c1.into_bigint().to_bytes_be()),
            U256::from_big_endian(&y.c0.into_bigint().to_bytes_be()),
        ],
    }
}

fn g1_contract_point_from_g1_affine(g1_affine: &G1Affine) -> G1PointContract {
    let x = g1_affine.x;
    let y = g1_affine.y;
    G1PointContract {
        x: U256::from_big_endian(&x.into_bigint().to_bytes_be()),
        y: U256::from_big_endian(&y.into_bigint().to_bytes_be()),
    }
}

fn g1_affine_from_g1_contract_point(
    g1_point: &G1PointContract,
) -> Result<G1Affine, ConversionError> {
    let mut x_bytes = [0u8; 32];
    g1_point.x.to_big_endian(&mut x_bytes);
    let mut y_bytes = [0u8; 32];
    g1_point.y.to_big_endian(&mut y_bytes);
    let x = Fq::from_be_bytes_mod_order(&x_bytes);
    let y = Fq::from_be_bytes_mod_order(&y_bytes);
    let point = G1Affine::new_unchecked(x, y);
    if !point.is_on_curve() {
        return Err(ConversionError::G1Point(
            "Point is not on curve".to_string(),
        ));
    }
    if !point.is_in_correct_subgroup_assuming_on_curve() {
        return Err(ConversionError::G1Point(
            "Point is not on correct subgroup".to_string(),
        ));
    }
    Ok(point)
}

fn g2_affine_from_g2_contract_point(
    g2_point: &G2PointContract,
) -> Result<G2Affine, ConversionError> {
    let mut x1_bytes = [0u8; 32];
    g2_point.x[1].to_big_endian(&mut x1_bytes);
    let mut x0_bytes = [0u8; 32];
    g2_point.x[0].to_big_endian(&mut x0_bytes);
    let x = Fp2::new(
        Fq::from_be_bytes_mod_order(&x1_bytes),
        Fq::from_be_bytes_mod_order(&x0_bytes),
    );
    let mut y1_bytes = [0u8; 32];
    g2_point.y[1].to_big_endian(&mut y1_bytes);
    let mut y0_bytes = [0u8; 32];
    g2_point.y[0].to_big_endian(&mut y0_bytes);
    let y = Fp2::new(
        Fq::from_be_bytes_mod_order(&y1_bytes),
        Fq::from_be_bytes_mod_order(&y0_bytes),
    );
    let point = G2Affine::new_unchecked(x, y);
    if !point.is_on_curve() {
        return Err(ConversionError::G2Point(
            "Point is not on curve".to_string(),
        ));
    }
    if !point.is_in_correct_subgroup_assuming_on_curve() {
        return Err(ConversionError::G2Point(
            "Point is not on correct subgroup".to_string(),
        ));
    }

    Ok(point)
}
