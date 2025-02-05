# Data types


## Dictionary, array, set

The system uses the three similar types (array, set, dictionary). While all of them use [a merkletree](./merkletree.md) under the hood, each of them uses it in a specific way:
- **dictionary**: the user original keys and values are hashed to be used in the leaf.
    - `leaf.key=hash(original_key)`
    - `leaf.value=hash(original_value)`
- **array**: the elements are placed at the value field of each leaf, and the key field is just the array index (integer)
    - `leaf.value=original_value` 
    - `leaf.key=i` 
- **set**: the value field of the leaf is unused, and the key contains the hash of the element
    -  `leaf.key=hash(original_value)`
    - `leaf.value=0`

In the three types, the merkletree under the hood allows to prove inclusion & non-inclusion of the particular entry of the {dictionary/array/set} element.
