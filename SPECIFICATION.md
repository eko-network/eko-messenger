# Eko-messenger

Version: 0.0.6

## Overview

Eko-messenger is a federated, decentralized, end-to-end encrypted messaging protocol built on top of [ActivityPub](https://www.w3.org/TR/activitypub/). It enables interpolation between ActivityPub servers while ensuring message encryption using the [Messaging Layer Security (MLS) Protocol](https://datatracker.ietf.org/doc/rfc9420/).

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
   1. All message content is encrypted on device using the MLS Protocol.

This document defines the eko-messenger protocol. Implementation-specific optimizations and guarantees are described separately.

## Terminology

* **User**: A human participant represented as an ActivityPub [Actor](https://www.w3.org/TR/activitypub/#actors). A User is a collection of one or more Devices.
* **Device/Client**: A cryptographic endpoint belonging to a User. Each device is a unique leaf in an MLS group tree.
* **Device ID**: A stable identifier for a Device. May be temporary (i.e. browser session).  
* **KeyPackage**: An MLS KeyPackage (as defined in RFC 9420) containing the cryptographic keys and parameters required to add a Device to a group.
* **PrivateMessage**: An MLS encrypted application message addressed to a group.
* **Welcome**: An MLS message used to invite a new member to a group.
* **Commit**: An MLS message used to initiate a new epoch for a group that instructs group members to apply the following Proposals.
* **Client-to-Server (C2S)**: Communication between a client/device and its home server.  
* **Server-to-Server (S2S)**: Federated communication between ActivityPub servers.
* **Group**: An MLS group consisting of multiple Devices. All communication in eko-messenger (including 1:1 chats) occurs within a Group.
* **Group ID**: A stable identifier for an MLS group.
* **Epoch**: The current version of the MLS group state.
* **EncryptedGroupState**: An opaque, end-to-end encrypted representation of a Group State (MLS context and ratchet tree), stored by the server for device synchronization.

## ActivityPub Model

### Users

* Each User is represented as an ActivityPub Actor (typically of type `Person`) with a standard inbox and outbox.

#### KeyPackage

Example: `KeyPackage` object  
```json  
{
  "@context": "https://eko.network/ns",
  "type": "KeyPackage",
  "id": "https://eko.network/user/user1/keyPackage/<hash>",
  "deviceId": "<device-id>",
  "value": "base64-encoded-mls-key-package"
}
```

### KeyCollection

To facilitate the distribution of `KeyPackage`s we define `KeyCollection`, a specialized type of Collection. Unlike standard Collections it is optimized for "Pop" semantics, where retrieving an item implies the consumption of said object. In MLS, these are used by other clients to add this device to a group.

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
      "KeyCollection": "https://eko.network/ns#KeyCollection"
    }
  ],
  "id": "https://example.com/alice/device/1/keys",
  "type": "KeyCollection",
  "attributedTo": "https://example.com/alice/device/1"
}
```
#### Access
External actors MUST NOT be able to read or browse the collection. External actors obtain a `KeyPackage` by performing an authenticated `POST` to the `KeyCollection` ID.

#### `Add` activity

The owner of the collection may add one or more `KeyPackage`s to the collection.

```json
{
  "type": "Add",
  "actor": "https://example.com/alice",
  "object": [
    {
      "type": "KeyPackage",
      "value": "..."
    }
  ],
  "target": "https://example.com/alice/device/1/keys"
}
```

#### Claiming a KeyPackage

To obtain key material for another user's device, a client performs an authenticated `POST` request to the `KeyCollection` URL.

Upon receiving a `POST` request, the server SHOULD:
* Select an available `KeyPackage` from the collection.
* Atomically remove the selected `KeyPackage`.
* Return the selected `KeyPackage` in the response body.
* Return a `410 Gone` or `404 Not Found` if no packages are available.

### Devices

* Each Actor exposes a `Devices` collection containing references to `AddDevice` and `RevokeDevice` objects forming a hash chain.
* A User's cryptographic presence in a group is the sum of their active Devices.
* Device Lifecycle  
  * **Add device**: The client issues a Create activity for an `AddDevice` object. When a new device is added, the user's existing devices SHOULD add the new device to all active MLS groups by issuing a `Commit` (with the new device's `KeyPackage`) and sending a `Welcome` message.
  * **Remove device**: The client issues a Create activity for a `RevokeDevice` object. Remaining devices in the user's active groups MUST issue a `Commit` to remove the revoked device's leaf from the MLS trees.

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
  "did": "urn:uuid:<uuid>",
  "eko:keyCollection": "https://eko.network/user/user1/device/<uuid>/keys",
  "identityKey": "<device Ed25519/P-256 publicKey>",
  "proof": {
    "type": "DataIntegrityProof",
    "cryptosuite": "eddsa-jcs-2022",
    "verificationMethod": "https://eko.network/user/user1#main-key",
    "proofPurpose": "assertionMethod",
    "proofValue": "z...."
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
    "cryptosuite": "eddsa-jcs-2022",
    "verificationMethod": "https://eko.network/user/user1#main-key",
    "proofPurpose": "assertionMethod",
    "proofValue": "z...."
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

All MLS encrypted messages:
* Targets *one* Group.
* `to` is the list of Users the server must fanout the message to.
* Contains one or more MLS messages (PrivateMessage, Welcome, or Commit).
* Is delivered as a single ActivityPub Create activity.
* `notify` is an optional field set by the client to hint whether or not the server should notify the recipient.
* `expires` is an optional field. If the server is unable to deliver the message before it expires, it should give up and discard the message. This is useful for transient activities such as a typing indicator.

Example: User sending a `PrivateMessage`
```json 
{
  "@context": "https://www.w3.org/ns/activitystreams",
  "type": "Create",
  "actor": "https://eko.network/user/user1",
  "to": "https://other.network/user/user2",
  "object": {
    "id": "https://eko.network/messages/id",
    "published": "2026-01-29T19:30:00Z",
    "notify": true,
    "type": "PrivateMessage",
    "content": "base64-encoded-ciphertext"
    ]
  }
}
```

Example: User sending a `Welcome` message
```json  
{
  "@context": "https://www.w3.org/ns/activitystreams",
  "type": "Create",
  "actor": "https://eko.network/user/user1",
  "to": "https://other.network/user/user2",
  "object": {
    "id": "https://eko.network/messages/id-welcome",
    "type": "Welcome",
    "content": "base64-encoded-welcome-message"
  }
}
```

#### Delivered
To support transience, upon receiving an MlsEnvelope, the client MUST respond with a `Delivered` activity. Upon receiving a `Delivered` Activity the server MUST remove that device's message entry or the entire envelope if delivered.
Example: a `Delivered` Activity
```json  
{
  "@context": "https://www.w3.org/ns/activitystreams",
  "id": "https://eko.network/activities/id",
  "type": "Delivered",
  "actor": "https://eko.network/user/user2",
  "to": "https://eko.network/user/user1",
  "object": "https://eko.network/messages/id"
}
```
## Group Messaging

This section defines end-to-end encrypted group messaging using the MLS protocol.

Group membership, state transitions, and message encryption are handled by the MLS protocol as defined in [RFC 9420](https://datatracker.ietf.org/doc/rfc9420/).
Servers are intentionally blind to group semantics and MUST NOT interpret, validate, or enforce group state.

### Group

A Group represents an encrypted conversation between multiple members (Devices).
- Identified by a stable `groupId`. MUST be a 128-bit UUID.
- **1:1 Messaging**: In eko-messenger, 1:1 messaging is implemented as an MLS group containing all devices of exactly two Users.
- **Group Messaging**: A group containing devices of two or more Users.

### Group State
<!-- Leaving here but unsure when/if this is needed or how/what information should be stored -->
Each client participating in a Group maintains a local MLS Group State.

#### Server Encrypted Group State
To support device synchronization and recovery, clients MAY upload encrypted snapshots of MLS Group State (including the ratchet tree and group context) to their home server.

Example: Server Visible EncryptedGroupState
```json
{
  "type": "EncryptedGroupState",
  "id": "https://eko.network/user/alice/groupState/<group-id>",
  "groupId": "urn:uuid:<group-id>",
  "epoch": 7,
  "mediaType": "application/mls-group-state",
  "encoding": "base64",
  "content": "<base64-encoded-ciphertext>"
}
```

### Sending a Group Message

1. Client prepares an ActivityPub object.
2. Client encrypts the object into an MLS `PrivateMessage` using the current MLS group state.
3. The `PrivateMessage` is sent to the inboxes of all group members' servers.

### Group Operations (Add/Remove)

Group operations like adding or removing members are performed using MLS `Commit` and `Welcome` messages.

1. **Adding a User**:
   - To add a User, the adding client MUST add **all** active devices of that User.
   - The adding client fetches a `KeyPackage` for each of the new User's devices from their `KeyCollection`.
   - The adding client creates an MLS `Commit` (adding all the devices) and the corresponding `Welcome` messages.
   - The `Commit` is sent to existing members.
   - The `Welcome` messages are sent to each of the new User's devices.

2. **Removing a User**:
   - To remove a User, the removing client MUST remove **all** devices associated with that User's Actor from the MLS group.
   - The removing client creates an MLS `Commit` removing the target devices.
   - The `Commit` is sent to the remaining members.

## Encrypted Content

All encrypted messages MUST encrypt a complete ActivityPub activity. Upon decryption, clients MUST process the content as if it were received directly from an ActivityPub inbox.

### Supported Activities

* Typing # special transient activity?  
* Read
* Create
* Update
* Delete

### Supported Objects

* Note  
* EmojiReact  
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
* Group messages must contain the `groupId` and `epoch`.

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
1. Fetches each recipient’s `KeyCollection` if adding new members or starting a group.
2. Encrypts the message using the MLS protocol (resulting in a `PrivateMessage`, `Welcome`, or `Commit`).
3. Creates an `MlsEnvelope` containing the MLS message(s).
4. POSTs a Create activity with the `MlsEnvelope` to its outbox.

### Receive Message

#### Message
1. Decrypt the MLS message using the local group state.
2. Read the decrypted content as ActivityPub.

## Server-to-Server Protocol (S2S)

### Send Message

When the server receives an `MlsEnvelope` message in the User’s inbox:

1. Server delivers the envelope to the receiver’s inbox.  
   1. Synchronously if the receiver is on the User’s homeserver.  
   1. Asynchronously if on an external server.

### Receive Message

When a server receives an `MlsEnvelope`, it SHOULD:

1. ACK the delivery.  
1. Verify the envelope is validly formatted.
1. Put the message in the receiving User’s inbox.

## E2E Encryption

Eko-messenger uses the **Messaging Layer Security (MLS)** protocol for end-to-end encryption. MLS provides efficient group key agreement with forward secrecy and post-compromise security.

For details on the MLS protocol, see [RFC 9420](https://datatracker.ietf.org/doc/rfc9420/).

### Ciphersuites
Clients SHOULD support the following MLS ciphersuites:
- `MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519`
- `MLS_128_DHKEMP256_AES128GCM_SHA256_P256`
<!-- do others? -->

## Key Management

### Identity Keys
Each device has a stable Ed25519 or P-256 identity key used to sign MLS `KeyPackage`s and authenticate the device.

### Key Packages
Devices publish MLS `KeyPackage`s to their `KeyCollection`. These packages are consumed by other clients to add the device to an MLS group.


## Trust Model and Limitations

* Servers are trusted to maintain the device list and correct keys.
  * See Federated Key Transparency work.
* Servers may store encrypted Group State for the purpose of device synchronization. Servers are not trusted to read, interpret, or modify Group State contents. Compromise of a server MUST NOT reveal group membership, cryptographic keys, or message content.

## eko-messenger Implementation Guarantees

The following are eko-messenger-specific behaviors and are not required by the protocol:

### Message Storage

* Messages are never stored after a client reads its inbox.

### Message Ordering

MLS provides an ordered delivery guarantee within the context of a group epoch. Messages within the same epoch are ordered by the sequence in which they were applied to the group state.

* Clients MUST process MLS `Commit` messages to maintain synchronization of the group state and message ordering.
* No global ordering guarantees across different groups or users. We will server timestamp messages to provide a client’s relative message ordering.  
* Clients MAY apply local heuristics for ordering.

### Push Notifications

* Will be implemented, but is handled out-of-band and is not part of the protocol.

# TODO's
* Alice lost all her devices (for instance, she deletes the app and its data). What does recovery look like.
