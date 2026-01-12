# Hash Chain Proposal

## Problem

In our model, when a user sends a message, it is possible for either the sender's server or the reciver's server to respond with a `Reject`. Upon reciving a `Reject` the sender must refresh their list of keys in order to maintain syncronization for both sender and reciver. This exchange creates an opertunity for a malicious server. A malicious server can generate its own prekeybundle for the sender to pick-up on refresh, allowing the server to eavsdrop on the conversation.

## Requirements
To mitigate this attack, the sender must verify that all cryptographic keys originate from valid devices.

A device is considered valid if and only if:

1. It has not been revoked by any other valid device, and

2. It has been approved by at least one valid device, or it is the initial device and is therefore implicitly trusted.

## Hash Chain Implementation

The hash chain is an append-only data structure featuring two events: `Approve` and `Revoke`.

```json
{
    "type": "Approve | Revoke",
    "content": {
        "prev": "<hash of previous event>"
        "did": "<device id of approved/rejected device>"
        "identity": "<identity key of approved/rejected device>"
    }
    "signature": "<signature of content>"
}
```

The signature of every node must be from a valid device, except for the first node which is self-signed.

#### Verification
In order to verify current valid devices, and observer would pull the full chain and replay events from the beginning to arrive at final state.
#### Gossip
It is possible that a malicious server will fork the tail of the chain. This could be used to hide revocations of compromised devices from a sender. To mitigate this, clients should gossip, comparing the hash of the final node to ensure they all have the same state. This gossip can be embedded in some or all messages.
#### Recovery
It is possible a user will lose access to all valid devices. Their server should allow them to reset their hash chain. The conditions for reset are left to the implementor.
#### Man In The Middle
For the first enrolled device, or after a reset, there is no protection against a malicious server. The users clients should notify them of changed identity and the users must verify a hash of all participants public keys out-out-of-band.

## Alternatives
### 1. Merkel Log with Sparse Merkel Tree

Similar to hash chain, more complex data structure.

#### Pros:
* Prevents needing to replay the full chain - O(log(n)) Verification.
* In some cases, may prevent the need to share full tree, allowing more privacy.

#### Cons:
* Requires two complex data structures
* Verification requires a Merkel proof for every device in chain, may outweigh reply cost.
* Sparse Merkel tree requires place for every possible hash, requiring a hard cap on registered devices.

### 2. Primary Device

All device changes must be done by a single primary device.

#### Pros:
* Less stored information.
* Don't need to make history public.
* Simple to verify.

#### Cons:
* May be less ergonomic to verify since you need primary device.
* Primary device cannot be changed.
* Unrecoverable without hard reset if primary device is compromised.

### 3. Shared Key

All devices share a private key.

#### Pros:
* Very simple, easy to have good UX.

#### Cons:
* Single point of failure, one compromised device requires full reset.

