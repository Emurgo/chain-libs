use super::transaction::*;
use crate::account;
use crate::block::HeaderHash;
use crate::key::{
    deserialize_public_key, deserialize_signature, serialize_public_key, serialize_signature,
    AccountSecretKey, AccountSignature, SpendingPublicKey, SpendingSecretKey, SpendingSignature,
};
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;
use chain_crypto::{Ed25519Bip32, PublicKey, Signature, Verification};

/// Structure that proofs that certain user agrees with
/// some data. This structure is used to sign `Transaction`
/// and get `SignedTransaction` out.
///
/// It's important that witness works with opaque structures
/// and may not know the contents of the internal transaction.
#[derive(Debug, Clone)]
pub enum Witness {
    Utxo(SpendingSignature<WitnessUtxoData>),
    Account(SpendingSignature<WitnessAccountData>),
    OldUtxo(
        PublicKey<Ed25519Bip32>,
        Signature<WitnessUtxoData, Ed25519Bip32>,
    ),
}

impl PartialEq for Witness {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (Witness::Utxo(s1), Witness::Utxo(s2)) => s1.as_ref() == s2.as_ref(),
            (Witness::Account(s1), Witness::Account(s2)) => s1.as_ref() == s2.as_ref(),
            (Witness::OldUtxo(p1, s1), Witness::OldUtxo(p2, s2)) => {
                s1.as_ref() == s2.as_ref() && p1 == p2
            }
            (_, _) => false,
        }
    }
}
impl Eq for Witness {}

pub struct WitnessUtxoData(Vec<u8>);

impl WitnessUtxoData {
    pub fn new(block0: &HeaderHash, transaction_id: &TransactionId) -> Self {
        let mut v = Vec::with_capacity(65);
        v.extend_from_slice(block0.as_ref());
        v.extend_from_slice(transaction_id.as_ref());
        WitnessUtxoData(v)
    }
}

impl AsRef<[u8]> for WitnessUtxoData {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

pub struct WitnessAccountData(Vec<u8>);

impl WitnessAccountData {
    pub fn new(
        block0: &HeaderHash,
        transaction_id: &TransactionId,
        spending_counter: &account::SpendingCounter,
    ) -> Self {
        let mut v = Vec::with_capacity(65);
        v.push(WITNESS_TAG_ACCOUNT);
        v.extend_from_slice(block0.as_ref());
        v.extend_from_slice(transaction_id.as_ref());
        v.extend_from_slice(&spending_counter.to_bytes());
        WitnessAccountData(v)
    }
}

impl AsRef<[u8]> for WitnessAccountData {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Witness {
    /// Creates new `Witness` value.
    pub fn new_utxo(
        block0: &HeaderHash,
        transaction_id: &TransactionId,
        secret_key: &SpendingSecretKey,
    ) -> Self {
        Witness::Utxo(SpendingSignature::generate(
            secret_key,
            &WitnessUtxoData::new(block0, transaction_id),
        ))
    }

    pub fn new_account(
        block0: &HeaderHash,
        transaction_id: &TransactionId,
        spending_counter: &account::SpendingCounter,
        secret_key: &AccountSecretKey,
    ) -> Self {
        Witness::Account(AccountSignature::generate(
            secret_key,
            &WitnessAccountData::new(block0, transaction_id, spending_counter),
        ))
    }

    // Verify the given `TransactionId` using the witness.
    pub fn verify_utxo(
        &self,
        public_key: &SpendingPublicKey,
        block0: &HeaderHash,
        transaction_id: &TransactionId,
    ) -> Verification {
        match self {
            Witness::OldUtxo(_xpub, _signature) => unimplemented!(),
            Witness::Utxo(signature) => {
                signature.verify(public_key, &WitnessUtxoData::new(block0, transaction_id))
            }
            Witness::Account(_) => Verification::Failed,
        }
    }
}

const WITNESS_TAG_OLDUTXO: u8 = 0u8;
const WITNESS_TAG_UTXO: u8 = 1u8;
const WITNESS_TAG_ACCOUNT: u8 = 2u8;

impl property::Serialize for Witness {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        //use chain_core::property::Serialize;

        let mut codec = Codec::new(writer);
        match self {
            Witness::OldUtxo(xpub, sig) => {
                codec.put_u8(WITNESS_TAG_OLDUTXO)?;
                serialize_public_key(xpub, &mut codec)?;
                serialize_signature(sig, &mut codec)
            }
            Witness::Utxo(sig) => {
                codec.put_u8(WITNESS_TAG_UTXO)?;
                serialize_signature(sig, codec.into_inner())
            }
            Witness::Account(sig) => {
                codec.put_u8(WITNESS_TAG_ACCOUNT)?;
                serialize_signature(sig, codec.into_inner())
            }
        }
    }
}

impl Readable for Witness {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        match buf.get_u8()? {
            WITNESS_TAG_OLDUTXO => {
                let xpub = deserialize_public_key(buf)?;
                let sig = deserialize_signature(buf)?;
                Ok(Witness::OldUtxo(xpub, sig))
            }
            WITNESS_TAG_UTXO => deserialize_signature(buf).map(Witness::Utxo),
            WITNESS_TAG_ACCOUNT => deserialize_signature(buf).map(Witness::Account),
            i => Err(ReadError::UnknownTag(i as u32)),
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    #[derive(Clone)]
    pub struct TransactionSigningKey(pub SpendingSecretKey);

    impl std::fmt::Debug for TransactionSigningKey {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "TransactionSigningKey(<secret-key>)")
        }
    }

    impl Arbitrary for TransactionSigningKey {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use rand_chacha::ChaChaRng;
            use rand_core::SeedableRng;
            let mut seed = [0; 32];
            for byte in seed.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            let mut rng = ChaChaRng::from_seed(seed);
            TransactionSigningKey(SpendingSecretKey::generate(&mut rng))
        }
    }

    impl Arbitrary for Witness {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Witness::Utxo(SpendingSignature::arbitrary(g))
        }
    }

    quickcheck! {

        /// ```
        /// \forall w=Witness(tx) => w.verifies(tx)
        /// ```
        fn prop_witness_verifies_own_tx(sk: TransactionSigningKey, tx:TransactionId, block0: HeaderHash) -> bool {
            let pk = sk.0.to_public();
            let witness = Witness::new_utxo(&block0, &tx, &sk.0);
            witness.verify_utxo(&pk, &block0, &tx) == Verification::Success
        }
    }
}
