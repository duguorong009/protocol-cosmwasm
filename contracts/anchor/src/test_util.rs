use ark_bn254::Bn254;
use ark_ff::{BigInteger, PrimeField};
use arkworks_circuits::setup::{
    common::{AnchorProof, Leaf},
    anchor::{
        setup_leaf_with_privates_raw_x5_4, setup_leaf_x5_4, setup_proof_x5_4, AnchorProverSetup
    },
};
use arkworks_utils::utils::common::{ setup_params_x5_3, setup_params_x5_4, Curve};

// wasm-utils dependencies
use wasm_utils::{
    proof::{generate_proof_js, JsProofInput, AnchorProofInput, ProofInput},
    types::{Backend, Curve as WasmCurve},
};

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct Element(pub [u8; 32]);

impl Element {
    fn to_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn from_bytes(input: &[u8]) -> Self {
        let mut buf = [0u8; 32];
        buf.copy_from_slice(input);
        Self(buf)
    }
}

type Bn254Fr = ark_bn254::Fr;

type ProofBytes = Vec<u8>;
type RootsElement = Vec<Element>;
type NullifierHashElement = Element;
type LeafElement = Element;


// merkle proof path legth
// TreeConfig_x5, x7 HEIGHT is hardcoded to 30
pub const TREE_DEPTH: usize = 30;
pub const M: usize = 2;
pub type AnchorSetup30_2 = AnchorProverSetup<Bn254Fr, M, TREE_DEPTH>;


pub fn setup_environment(curve: Curve) -> (Vec<u8>, Vec<u8>) {
    match curve {
        Curve::Bn254 => {
            let pk_bytes = include_bytes!(
                "../../../protocol-substrate-fixtures/fixed-anchor/bn254/x5/proving_key_uncompressed.bin"
            );
            let vk_bytes = include_bytes!(
                "../../../protocol-substrate-fixtures/fixed-anchor/bn254/x5/verifying_key.bin"
            );

            (pk_bytes.to_vec(), vk_bytes.to_vec())
        }
        Curve::Bls381 => {
            unimplemented!()
        }
    }
}


pub fn setup_zk_circuit(
	curve: Curve,
	recipient_bytes: Vec<u8>,
	relayer_bytes: Vec<u8>,
	commitment_bytes: Vec<u8>,
	pk_bytes: Vec<u8>,
	chain_id: u64,
	fee_value: u128,
	refund_value: u128,
) -> (ProofBytes, RootsElement, NullifierHashElement, LeafElement) {
	let rng = &mut ark_std::test_rng();

	match curve {
		Curve::Bn254 => {
			let Leaf { secret_bytes, nullifier_bytes, leaf_bytes, nullifier_hash_bytes } =
				setup_leaf_x5_4::<Bn254Fr, _>(Curve::Bn254, chain_id.into(), rng).unwrap();
			let leaves = vec![leaf_bytes.clone()];
			let leaves_f = vec![Bn254Fr::from_le_bytes_mod_order(&leaf_bytes)];
			let index = 0;

			let params3 = setup_params_x5_3::<Bn254Fr>(curve);
			let params4 = setup_params_x5_4::<Bn254Fr>(curve);
			let anchor_setup = AnchorSetup30_2::new(params3, params4);
			let (tree, _) = anchor_setup.setup_tree_and_path(&leaves_f, index).unwrap();
			let roots_f = [tree.root().inner(); M];
			let roots_raw = roots_f.map(|x| x.into_repr().to_bytes_le());

			let AnchorProof { proof, roots_raw, .. } = setup_proof_x5_4::<Bn254, _>(
				curve,
				chain_id.into(),
				secret_bytes,
				nullifier_bytes,
				leaves,
				index,
				roots_raw.clone(),
				recipient_bytes,
				relayer_bytes,
				commitment_bytes,
				fee_value,
				refund_value,
				pk_bytes,
				rng,
			)
			.unwrap();

			let roots_element = roots_raw.iter().map(|x| Element::from_bytes(&x)).collect();
			let nullifier_hash_element = Element::from_bytes(&nullifier_hash_bytes);
			let leaf_element = Element::from_bytes(&leaf_bytes);

			(proof, roots_element, nullifier_hash_element, leaf_element)
		},
		Curve::Bls381 => {
			unimplemented!()
		},
	}
}
pub fn setup_wasm_utils_zk_circuit(
	curve: Curve,
	recipient_bytes: Vec<u8>,
	relayer_bytes: Vec<u8>,
	commitment_bytes: [u8; 32],
	pk_bytes: Vec<u8>,
	chain_id: u128,
	fee_value: u128,
	refund_value: u128,
) -> (
	Vec<u8>,      // proof bytes
	Vec<Element>, // roots
	Element,      // nullifier_hash
	Element,      // leaf
) {
	match curve {
		Curve::Bn254 => {
			let note_secret = "7e0f4bfa263d8b93854772c94851c04b3a9aba38ab808a8d081f6f5be9758110b7147c395ee9bf495734e4703b1f622009c81712520de0bbd5e7a10237c7d829bf6bd6d0729cca778ed9b6fb172bbb12b01927258aca7e0a66fd5691548f8717";
			let raw = hex::decode(&note_secret).unwrap();

			let secret = hex::decode(&note_secret[0..32]).unwrap();
			let nullifier = hex::decode(&note_secret[32..64]).unwrap();
			let leaf = setup_leaf_with_privates_raw_x5_4::<Bn254Fr>(
				curve,
				secret.to_vec(),
				nullifier.to_vec(),
				chain_id,
			)
			.unwrap();

			let leaves = vec![leaf.leaf_bytes.clone()];
			let leaves_f = vec![Bn254Fr::from_le_bytes_mod_order(&leaf.leaf_bytes)];
			let index = 0;

			let params3 = setup_params_x5_3::<Bn254Fr>(curve);
			let params4 = setup_params_x5_4::<Bn254Fr>(curve);
			let anchor_setup = AnchorSetup30_2::new(params3, params4);
			let (tree, _) = anchor_setup.setup_tree_and_path(&leaves_f, index).unwrap();
			let roots_f = [tree.root().inner(); M];
			let roots_raw = roots_f.map(|x| x.into_repr().to_bytes_le());

			let mixer_proof_input = AnchorProofInput {
				exponentiation: 5,
				width: 4,
				curve: WasmCurve::Bn254,
				backend: Backend::Arkworks,
				secrets: secret.to_vec(),
				nullifier: nullifier.to_vec(),
				recipient: recipient_bytes,
				relayer: relayer_bytes,
				pk: pk_bytes,
				refund: refund_value,
				fee: fee_value,
				chain_id,
				leaves,
				leaf_index: index,
				roots: roots_raw.to_vec(),
				commitment: commitment_bytes,
			};
			let js_proof_inputs = JsProofInput { inner: ProofInput::Anchor(mixer_proof_input) };
			let proof = generate_proof_js(js_proof_inputs).unwrap();

			let root_elements = proof.roots.iter().map(|root| Element::from_bytes(&root)).collect();
			let nullifier_hash_element = Element::from_bytes(&proof.nullifier_hash);
			let leaf_element = Element::from_bytes(&proof.leaf);

			(proof.proof, root_elements, nullifier_hash_element, leaf_element)
		},
		Curve::Bls381 => {
			unimplemented!()
		},
	}
}

/// Truncate and pad 256 bit slice in reverse
pub fn truncate_and_pad_reverse(t: &[u8]) -> Vec<u8> {
    let mut truncated_bytes = t[12..].to_vec();
    truncated_bytes.extend_from_slice(&[0u8; 12]);
    truncated_bytes
}
