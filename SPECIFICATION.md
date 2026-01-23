# Eko-messenger

Version: 0.0.2

## Overview

Eko-messenger is a federated, decentralized, end-to-end encrypted messaging protocol built on top of [ActivityPub](https://www.w3.org/TR/activitypub/). It enables interpolation between ActivityPub servers while ensuring message encryption using the [Signal Protocol](https://signal.org/docs/).

### Purpose

E2E encrypted apps typically use a centralized server for storing and/or forwarding encrypted messages, like Signal and WhatsApp. We want to *extend* the standard protocol for federated applications (ActivityPub) to support end-to-end encryption.

eko-messenger is designed to:

* Reuse existing ActivityPub infrastructure for federation.  
* Treat devices as first-class cryptographic endpoints.  
* Avoid server-side message storage beyond transient delivery.  
* Remain extensible to different server and client implementations.

### Goals

1. Federation  
   1. Open protocol: any server implementing this specification may participate.  
   2. Uses ActivityPub for both C2S and S2S communication.  
   3. Compatible with existing ActivityPub federation semantics.  
2. Decentralization  
   1. Achieves decentralization through federation.  
3. End-to-End Encrypted  
   1. All message content is encrypted on device using the Signal Protocol.

This document defines the eko-messenger protocol. Implementation-specific optimizations and guarantees are described separately.

## Terminology

* **User**: A human participant represented as an ActivityPub [Actor](https://www.w3.org/TR/activitypub/#actors).  
* **Device/Client**: A cryptographic endpoint belonging to a User. Each device independently participates in Signal sessions.  
* **Device ID**: A stable identifier for a Device. May be temporary (i.e. browser session).  
* **KeyPackage**: A published bundle of Public Identity Keys and PreKeys required to initiate a Signal session with a Device.  
* **SignalEnvelope**: An ActivityPub object containing encrypted messages for one or more destination Devices.  
* **Message**: A Signal encrypted payload addressed to a single Device.  
* **Client-to-Server (C2S)**: Communication between a client/device and its home server.  
* **Server-to-Server (S2S)**: Federated communication between ActivityPub servers.

## ActivityPub Model

### Users

* Each User is represented as an ActivityPub Actor (typically of type `Person`) with a standard inbox and outbox.

#### KeyPackage

Example: `KeyPackage` object  
```json  
{
  "@context": "https://eko.network/ns",
  "type": "KeyPackage",
  "id": "https://eko.network/user/user1/keyPackage/A",
  "deviceId": "<device-id>",
  "preKeyId": 1,
  "preKey": "base64-encoded",
  "signedPreKeyId": 1,
  "signedPreKey": "base64-encoded",
  "signedPreKeySignature": "base64-encoded"
}
```

### KeyCollection

To facilitate the distribution of `KeyPackage`s we define `KeyCollection`, a specialized type of Collection. Unlike standard Collections it is optimized for "Pop" semantics, where retrieving an item implies the consumption of said object.

#### Object
A `KeyCollection` must be owned by an Actor or a sub-entity such as a Device.

Properties:
* type: MUST be `KeyCollection`
* attributedTo: Device owning the collection.
```json
{
  "@context": [
    "https://www.w3.org/ns/activitystreams",
    {
      "KeyCollection": "TODO"
    }
  ],
  "id": "https://example.com/alice/device/1/keys",
  "type": "KeyCollection",
  "attributedTo": "https://example.com/alice/device/1",
}
```
#### Access
External actors MUST NOT be able to read or browse the collection. External actors may only interact with the collection through the `Take` activity.

#### `Add` activity

The owner of the collection may add one or more `KeyBundles` to the collection.

```json
{
  "type": "Add",
  "actor": "https://example.com/alice",
  "object": [
    {
      "type": "KeyPackage",
      "value": "..."
    },
    {
      "type": "KeyPackage",
      "value": "..."
    }
  ],
  "target": "https://example.com/alice/device/1/keys"
}
```

#### `Take` activity

We define an activity `Take` which user may use to interact with another users `KeyCollection`. To obtain key material for another user, a user will Post a `Take` to their inbox.

```json
{
  "@context": [
    "https://www.w3.org/ns/activitystreams",
    {
      "Take": "TODO"
    }
  ],
  "type": "Take",
  "actor": "https://example.com/bob",
  "object": "https://example.com/alice/device/1/keys",
}
```

Upon receiving a `Take` activity, the server SHOULD:
* Select a `KeyPackage` from the user's collection.
* If there are multiple key packages in the collection, atomically remove the selected `KeyPackage`.
* Return the selected `KeyPackage`.

### Devices

* Each Actor exposes a `Devices` collection containing references to `AddDevice` and `RevokeDevice` objects forming a hash chain. Each `AddDevice` object should contain a reference to a `KeyCollection`. 
* Device Lifecycle  
  * Add device: the client issues a [Create](https://www.w3.org/TR/activitystreams-vocabulary/#dfn-create) activity addressed to the `Devices` collection for a `AddDevice` object.  
  * Remove device: the client issues a [Create](https://www.w3.org/TR/activitystreams-vocabulary/#dfn-create) activity addressed to the `Devices` collection for a `Revoke` object.

#### AddDevice
```json
{
  "@context": [
    "https://www.w3.org/ns/activitystreams",
    "https://eko.network/ns"
  ],
  "type": "AddDevice",
  "id": "https://eko.network/user/devices/actions/<id>",
  "prev": "<hash of previous node or null if first node>",
  "did": 0,
  "eko:keyPackage": "https://eko.network/user/user1/keyPackage",
  "identityKey": "<device publicKey>",
  "registrationId": 1,
  "proof": {
    "type": "DataIntegrityProof",
    "cryptosuite": "xeddsa-2022",
    "verificationMethod": "did:eko:asdasd",
    "proofPurpose": "Authentication",
    "proof_value": "z....",
  }
}
```

#### RevokeDevice
```json
{
  "@context": [
    "https://www.w3.org/ns/activitystreams",
    "https://eko.network/ns"
  ],
  "type": "RevokeDevice",
  "id": "https://eko.network/user/devices/actions/<id>",
  "did": "urn:uuid:<uuid>",
  "prev": "<hash of previous node>",
  "proof": {
    "type": "DataIntegrityProof",
    "cryptosuite": "xeddsa-2022",
    "verificationMethod": "did:eko:asdasd",
    "proofPurpose": "Authentication",
    "proof_value": "z....",
  }
}
```
To compute the prev hash, clients and server MUST format the node in accordance with RFC 8785 and use SHA-256. To compute the signatures, the client MUST remove both the proof field and the id field, then format the remaining node in compliance with RFC 8785, signing with their identity key.

#### KeyPackages

Example: User with keyPackages collection  
```json  
{
  "@context": [
    "https://www.w3.org/ns/activitystreams",
    "https://eko.network/ns"
  ],
  "type": "Person",
  "id": "https://eko.network/user/user1",
  "preferredUsername": "user1",
  "inbox": "https://eko.network/user/user1/inbox",
  "outbox": "https://eko.network/user/user1/outbox",
  "eko:keyPackages": {
    "type": "Collection",
    "id": "https://eko.network/user/user1/keyPackages",
    "items": [
      "https://eko.network/user/user1/keyPackage/A",
      "https://eko.network/user/user1/keyPackage/B"
    ]
  }
}
```

### Messages

#### SignalEnvelope

All Signal encrypted messages are transported inside a `SignalEnvelope`.

* Targets *one* user (TODO: for groups, do we send N SignalEnvelopes for each user? Or, we probably want to wrap again into some SignalGroupEnvelope or smth, so the server can verify all users & devices are being sent a message).  
* Contains one encrypted Message per destination Device  
  * Encrypted messages are of `“messageType”: “message/signal”`.  
  * Message content is stored as a base64 datatype. When unencrypted, the content uses ActivityPub defined types.  
* Is delivered as a single ActivityPub Create activity.

Example: User sending a `SignalEnvelope`  
```json  
{
  "@context": "https://www.w3.org/ns/activitystreams",
  "type": "Create",
  "actor": "https://eko.network/user/user1",
  "to": [
    "https://other.network/user/user2"
  ],
  "object": {
    "type": [
      "Object",
      "SignalEnvelope"
    ],
    "mediaType": "message/signal",
    "encoding": "base64",
    "messages": [
      {
        "deviceId": "device-A",
        "content": "base64-encoded-ciphertext"
      },
      {
        "deviceId": "device-B",
        "content": "base64-encoded-ciphertext"
      }
    ]
  }
}
```

## Encrypted Content

All encrypted messages MUST encrypt a complete ActivityPub activity. Upon decryption, clients MUST process the content as if it were received directly from an ActivityPub inbox.

### Supported Objects

* Note  
* EmojiReact  
* Typing # special transient activity?  
* Image  
* Audio  
* Video

### Constraints

The following restrictions apply to content objects embedded in encrypted messages:

* `id`  
  * MUST be 128-bit uuid so it’s a unique, non-resolvable URI.  
* `content`  
  * For Note, this MUST contain the HTML content of the object.  
* `summary`  
  * Optional human-readable summary or description of the content.  
* `attachment`  
  * List of attachments in the message. Images and files SHOULD be encrypted with AES-256-GCM using a new key for each attachment. The attachment MUST include a digest field with the SHA-256 hash of the encrypted file NOT the original file.

Example: Sending Attachments  
```json  
[
  {
    "contentType": "image/jpeg",
    "encryption": "AES-256-GCM",
    "key": "b64_encoded_key",
    "url": "https://cdn.example.org/attachments/abc-123-xyz",
    "size": 1048576,
    "name": "image.jpg",
    "blurHash": "LAAwF",
    "digest": "sha256_hash"
  },
  {
    "contentType": "application/gzip",
    "encryption": "AES-256-GCM",
    "key": "b64_encoded_key",
    "url": "https://cdn.example.org/attachments/def-456-uvw",
    "size": 34023,
    "name": "file.tar.gz",
    "digest": "sha256_hash"
  }
]
```

* `inReplyTo`  
  * References the id of a content object previously delivered to the same conversation.

Example: Create activity  
```json  
{
  "@context": "https://www.w3.org/ns/activitystreams",
  "type": "Create",
  "id": "urn:uuid:<uuid>",
  "object": {
    "type": "Note",
    "id": "urn:uuid:<uuid>",
    "content": "Hello, World!"
  }
}
```  
Example: Update activity  
```json  
{
  "@context": "https://www.w3.org/ns/activitystreams",
  "type": "Update",
  "id": "urn:uuid:<uid>",
  "object": {
    "type": "Note",
    "id": "urn:uuid:<uid>",
    "content": "Hello, World Universe!"
  }
}
```  
Example: Delete activity  
```json  
{
  "@context": "https://www.w3.org/ns/activitystreams",
  "type": "Delete",
  "id": "urn:uuid:<uid>",
  "object": "urn:uuid:<uid>"
}
```

## Client-to-Server Protocol (C2S)

### Send Message

When sending a message, the client:

1. Fetches each recipient’s `keyPackages` collection.  
1. Encrypts the message for each recipient’s Device using the Signal protocol.  
1. Creates a `SignalEnvelope` containing one encrypted Message per Device.  
1. POSTs a Create activity with the `SignalEvelop` to its outbox.

### Receive Message

#### Message

1. Decrypt message.  
1. Read as ActivityPub.

#### PartialDelivery

1. Re-pull Recipient’s `KeyPackages` and any keys the client does not have.  
1. Resend `SignalEnvelop` with remaining encrypted messages.

#### Reject

1. Re-pull Recipient’s KeyPackages and any keys the client does not have.  
1. Resend `SignalEnvelope` with new encrypted messages.

## Server-to-Server Protocol (S2S)

### Send Message

When the server receives a `SignalEnvelope` message in the User’s inbox:

1. Server delivers the envelope to the receiver’s inbox.  
   1. Synchronously if the receiver is on the User’s homeserver.  
   1. Asynchronously if on an external server.  
      1. Note: the external server may reject the `SignalEnvelope` if not all devices have an encrypted message.

### Receive Message

When a server receives a `SignalEnvelope`, it SHOULD:

1. ACK the delivery.  
1. Verify the envelope contains exactly one encrypted Message for each currently registered Device of the recipient User.  
1. If verification succeeds, the message is put in the receiving User’s inbox.  
   1. The server MAY send a Confirm activity to the sender’s inbox to confirm delivery.  
1. If verification fails, a Reject activity MUST be sent to the Sender’s inbox.

Example: Reject  
```json  
{
  "@context": [
    "https://www.w3.org/ns/activitystreams",
    "https://eko.network/ns"
  ],
  "type": "Reject",
  "actor": "https://other.network",
  "to": [
    "https://eko.network/user/user1"
  ],
  "object": "https://eko.network/user/signal/<envelope-id>",
  "summary": "SignalEnvelope rejected: encrypted messages missing for one or more recipient devices."
}
```

## E2E Encryption

- TODO: Signal encryption mechanism described here.

## Key Management

- TODO

## Trust Model and Limitations

* Servers are trusted to maintain the device list and correct keys.  
  * See Federated Key Transparency work.

## eko-messenger Implementation Guarantees

The following are eko-messenger-specific behaviors and are not required by the protocol:

### Message Storage

* Messages are never stored after a client reads its inbox.

### Message Ordering

Signal currently has no message ordering guarantees, and is a current [issue](https://community.signalusers.org/t/message-ordering/2581/56). Messages may arrive out of order.

* No global ordering guarantees. We will server timestamp messages to provide a client’s relative message ordering.  
* Clients MAY apply local heuristics for ordering.

### Push Notifications

* Will be implemented, but is handled out-of-band and is not part of the protocol.
