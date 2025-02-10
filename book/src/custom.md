# Custom statements and custom operations

<font color="red">TODO</font>: Settle on a design to protect against unwanted custom operations in upstream PODS.  One such design option is proposed in <font color="green">green</font>.

Users of the POD system can introduce _custom statements_ to express complex logical relations not available in the built-in statements.  Custom statements are built on top of existing statements (built-in or custom) using logical _custom operations_.

Every MainPod has room to introduce a number of _custom statements_ and _custom operations_. <font color="green"> As a security requirement, every custom operation introduced in a MainPod must output a custom statement introduced in the same MainPod. </font> When a custom statement is introduced in a MainPod, it becomes available for use in that POD and all PODs that inherit[^inherit] from it.

## Custom statements and IDs

<font color="green"> The public data in every MainPod includes a list of all custom statements and custom operations used in that POD and all ancestor PODs. </font>

A custom statement, like a built-in statement, is identified by a _name_ on the front end and an _identifier_ on the back end.  Both name and identifier must be unique -- that is, distinct from the names and identifiers of all custom statements in this POD and all ancestor PODs.  <font color="green"> The POD circuit verifies that the identifier is unique. </font>

## Custom operations

A custom operation [^operation] is a rule that allows one to deduce a custom statement from one or more existing statements according to a logical rule, described below.

> Note: Unlike built-in operations, it is not possible to perform arbitrary calculations inside a custom operation.

The syntax of a custom operation is best explained with an example:
| Args | Condition            | Output                      |
|------------|-----------------------------------------|----|
| pod: Origin, <br> good_boy_issuers: AnchoredKey::MerkleRoot, <br> receiver: AnchoredKey | ValueOf(AK(pod, "_type"), SIGNATURE), <br> Contains(good_boy_issuers, AK(pod,"_signer")), <br> Equals(AK(pod, "friend"), receiver) | GoodBoy(receiver, good_boy_issuers) |

A custom operation accepts as input a number of statements (the `Condition`); 
each statement has a number of arguments, which may be constants or anchored keys; and an [anchored key](./anchoredkeys.md) in turn can optionally be decomposed as a pair of an Origin and a Key.

In the example above, the anchored keys `good_boy_issuers` and `receiver` are not broken down, but `AK(pod, "_type"), AK(pod, "_signer"), AK(pod, "friend")` are.  The purpose of breaking them down, in this case, is to force the three anchored keys to come from the same pod.

In general, in the front-end language, the "arguments" to an operation define a list of identifiers with types.  Every statement in the "condition" must have valid arguments of the correct types: either constants, or identifiers defined in the "arguments".

In order to apply the operation, the user who wants to create a POD must give acceptable values for all the arguments.  The POD prover will substitute those values for all the statements in the "Condition" and check that all substituted statements previously appear in the POD.  If this check passes, the output statement is then a valid statement.

[^inherit]: What to call this?  One POD "inherits" from another?

[^operation]: In previous versions of these docs, "operations" were called "deduction rules".
