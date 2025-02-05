# POD value types
From the frontend perspective, POD values may be one of three[^type] types:
- `Integer`
- `String`
- `MerkleTree`

From the backend perspective, however, these types will all be encoded as a fixed number of field elements, the number being chosen so as to accommodate the `Integer` type as well as hashes to represent the `String` and `MerkleTree` types with the appropriate level of security.

In the case of the Plonky2 backend with 100 bits of security, all of these types are represented as 4 field elements, the output of the Poseidon hash function used there being

$$\texttt{HashOut<GoldilocksField>}\simeq\texttt{[GoldilocksField; 4]}.$$

## `Integer`
In the frontend, this type is none other than `u64`[^i64]. In the backend, it will be appropriately embedded into the codomain of the canonical hash function.

In the case of the Plonky2 backend, this is done by decomposing such an integer $x$ as
$$x = x_0 + x_1 \cdot 2^{32}$$
with $0 \leq x_0, x_1 < 2^{32}$ and representing it as
$$\texttt{map}\ \iota\ [x_0, x_1, 0, 0],$$
where $\iota:\mathbb{N}\cup\{0\}\rightarrow\texttt{GoldilocksField}$ is the canonical projection.

## `String`
In the frontend, this type corresponds to the usual `String`. In the backend, the string will be mapped to a sequence of field elements and hashed with the hash function employed there, thus being represented by its hash.

In the case of the Plonky2 backend, the string is converted to a sequence of bytes with the byte `0x01` appended as  padding, then the bytes are split into 7-byte chunks starting from the left, these chunks then being interpreted as integers in little-endian form, each of which is naturally an element of `GoldilocksField`, whence the resulting sequence may be hashed via the Poseidon hash function. Symbolically, given a string $s$, its hash is defined by

$$\texttt{poseidon}(\texttt{map}\ (\iota\circ\jmath_\texttt{le-bytes->int})\ \texttt{chunks}_7(\jmath_\texttt{string->bytes}(s)\ \texttt{++}\ [\texttt{0x01}])),$$

where `poseidon` is the Poseidon instance used by Plonky2, $\iota$ is as above, $\texttt{chunks}_{n}:[\texttt{u8}]\rightarrow [[\texttt{u8}]]$ is defined such that[^aux]

$$\texttt{chunks}_n(v) = \textup{if}\ v = [\ ]\ \textup{then}\ [\ ]\ \textup{else}\ [\texttt{take}_n v]\ \texttt{++}\ \texttt{chunks}_n(\texttt{drop}_n v),$$

the mapping $\jmath_\texttt{le-bytes->int}: [u8] \rightarrow{N}\cup\{0\}$ is given by

$$[b_0,\dots,b_{N-1}]\mapsto \sum_{i=0}^{N-1} b_i \cdot 2^{8i},$$

and $\jmath_\texttt{string->bytes}$ is the canonical mapping of a string to its UTF-8 representation.


## `MerkleTree`
In the front end, this type encapsulates a sparse Merkle tree. In the backend, this will be represented by the root hash of the tree, which will be of the type of the output of the hash function employed.

[^type]: <font color="red">TODO</font> In POD 1, there is the `cryptographic` type, which has the same type of the output of the hash function employed there. It is useful for representing arbitrary hashes. Do we want to expand our type list to include a similar type, which would correspond to the `HashOut` type in the case of Plonky2? This would not have a uniform representation in the frontend if we continue to be backend agnostic unless we fix the number of bits to e.g. 256, in which case we would actually need one more field element in the case of Plonky2.
[^i64]: <font color="red">TODO</font> Replace this with `i64` once operational details have been worked out.
[^aux]: Definitions of `drop` and `take` may be found [here](https://hackage.haskell.org/package/haskell98-2.0.0.3/docs/Prelude.html#v:drop) and [here](https://hackage.haskell.org/package/haskell98-2.0.0.3/docs/Prelude.html#v:take).
