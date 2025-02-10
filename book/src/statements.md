# Statements

A _statement_ is any sort of claim about the values of entries: for example, that two values are equal, or that one entry is contained in another.

Statements come in two types: _built-in_ and _custom_.  There is a short list of built-in statements (see below). [^builtin]
In addition, users can freely define custom statements.

From the user (front-end) perspective, a statement represents a claim about the values of some number of entries -- the statement can only be proved if the claim is true.  On the front end, a statement is identified by its _name_ (`ValueOf`, `Equal`, etc.).

From the circuit (back-end) perspective, a statement can be proved either:
- by direct in-circuit verification, or
- by an operation (aka deduction rule).
On the back end, a statement is identified by a unique numerical _identifier_.

## Built-in statements

The POD system has several builtin statements. These statements are associated to a reserved set of statement IDs.

```
ValueOf(key: AnchoredKey, value: ScalarOrVec)

Equal(ak1: AnchoredKey, ak2: AnchoredKey)

NotEqual(ak1: AnchoredKey, ak2: AnchoredKey)

Gt(ak1: AnchoredKey::Integer, ak2: AnchoredKey::Integer)

Lt(ak1: AnchoredKey::Integer, ak2: AnchoredKey::Integer)

GEq(ak1: AnchoredKey::Integer, ak2: AnchoredKey::Integer)

LEq(ak1: AnchoredKey::Integer, ak2: AnchoredKey::Integer)

SumOf(sum: AnchoredKey::Integer, arg1: AnchoredKey::Integer, arg2: 
AnchoredKey::Integer)

ProductOf(prod: AnchoredKey::Integer, arg1: AnchoredKey::Integer, arg2: AnchoredKey::Integer)

MaxOf(max: AnchoredKey::Integer, arg1: AnchoredKey::Integer, arg2: AnchoredKey::Integer)
```

The following statements relate to Merkle trees and compound types; they are explained in detail on a [separate page](./merklestatements.md).
```
Branches(parent: AnchoredKey::MerkleTree, left: AnchoredKey::MerkleTree, right: AnchoredKey::MerkleTree)

Leaf(node: AnchoredKey::MerkleTree, key: AnchoredKey, value: AnchoredKey)

IsNullTree(node: AnchoredKey::MerkleTree)

GoesLeft(key: AnchoredKey, depth: Value::Integer)

GoesRight(key: AnchoredKey, depth: Value::Integer)

Contains(root: AnchoredKey::MerkleTree, key: AnchoredKey, value: AnchoredKey)

MerkleSubtree(root: AnchoredKey::MerkleTree, node: AnchoredKey::MerkleTree)

MerkleCorrectPath(root: AnchoredKey::MerkleTree, node: AnchoredKey::MerkleTree, key: AnchoredKey, depth: Value::Integer)

Contains(root: AnchoredKey::MerkleTree, key: AnchoredKey, value: AnchoredKey)

NotContains(root: AnchoredKey::MerkleTree, key: AnchoredKey)

ContainsHashedKey(root: AnchoredKey::DictOrSet, key: AnchoredKey)

NotContainsHashedKey(root: AnchoredKey::DictOrSet, key: AnchoredKey)

ContainsValue(root: AnchoredKey::Array, value: AnchoredKey)
```


In the future, we may also reserve statement IDs for "precompiles" such as:
```
PoseidonHashOf(A.hash, B.preimage) // perhaps a hash_of predicate can be parametrized by an enum representing the hash scheme; rather than having a bunch of specific things like SHA256_hash_of and poseidon_hash_of etc.
```

```
EcdsaPrivToPubOf(A.pubkey, B.privkey)
```

### Built-in statements for entries of any type

A ```ValueOf``` statement asserts that an entry has a certain value.
```
ValueOf(A.name, "Arthur") 
```

An ```Equal``` statement asserts that two entries have the same value.  (Technical note: The circuit only proves equality of field elements; no type checking is performed.  For strings or Merkle roots, collision-resistance of the hash gives a cryptographic guarantee of equality.  However, note both Arrays and Sets are implemented as dictionaries in the backend; the backend cannot type-check, so it is possible to prove an equality between an Array or Set and a Dictionary.)
```
Equal(A.name, B.name)
```

An ```NotEqual``` statement asserts that two entries have different values.
```
NotEqual   (for arbitrary types)
```

##### Built-in Statements for Numerical Types
An ```Gt(x, y)``` statement asserts that ```x``` is an entry of type ```Integer```, ```y``` is an entry or constant of type ```Integer```, and ```x > y```.
```
Gt    (for numerical types only)
Gt(A.price, 100)
Gt(A.price, B.balance)
```

The statements ```Lt```, ```GEq```, ```Leq``` are defined analogously.

```SumOf(x, y, z)``` asserts that ```x```, ```y```, ```z``` are entries of type ```Integer```, and [^fillsum]

```ProductOf``` and ```MaxOf``` are defined analogously.

The two items below may be added in the future:
```
poseidon_hash_of(A.hash, B.preimage) // perhaps a hash_of predicate can be parametrized by an enum representing the hash scheme; rather than having a bunch of specific things like SHA256_hash_of and poseidon_hash_of etc.
```

```
ecdsa_priv_to_pub_of(A.pubkey, B.privkey)
```

##### Primitive Built-in Statements for Merkle Roots

[See separate page](./merklestatements.md).



[^builtin]: <font color="red">TODO</font> List of built-in statements is not yet complete.

[^fillsum]: <font color="red">TODO</font> Does sum mean x+y = z or x = y+z?