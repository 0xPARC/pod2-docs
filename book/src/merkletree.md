# MerkleTree

In the POD system, MerkleTrees are used to store the key-values of the POD.

- Each leaf contains a touple of `key` and `value`
- Leaf position is determined by the `key` content binary representation (little-endian)

![](img/merkletree0.png)

So for example, suppose we have the following data in a POD:
```js
{
	id: "11000...",
	kvs : {
		idNumber: "424242",
		dateOfBirth: 1169909384,
		userPk: 9876543210, // target user of this POD
		_signerPk: 1234567890, // signer of the POD
	}
}
```

The merkletree will contain the key values from the `kvs` field.

Suppose that the binary representation of the key `userPk` is `1011...`. This uniquely defines the leaf position that contains the public key of the authenticated user.

![](img/merkletree1.png)


## Proofs of inclusion and non-inclusion
Merkle proofs contain the siblings along the path from the leaf to the root, where the leaf position is determined by the key binary representation. 

The same approach works for proving that a key is not in the tree, and furthermore, that a value is not in the tree. The main difference is that it is proving that in the leaf position there is no key or value stored, and since leaf positions are deterministic based on the key, if at the position of the key `signerPk` the leaf is empty (non-existent), it means that the tree does not contain any leaf with that key.


## Temporary first version
The first iteration of the implementation uses a hash of the key-values concatenated, with the idea of replacing it by the MerkleTree approach described above.

## Resources
- [https://docs.iden3.io/publications/pdfs/Merkle-Tree.pdf](https://docs.iden3.io/publications/pdfs/Merkle-Tree.pdf)
- [https://eprint.iacr.org/2018/955](https://eprint.iacr.org/2018/955)
