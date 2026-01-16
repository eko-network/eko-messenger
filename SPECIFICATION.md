# Eko-messenger

Version: 0.0.4

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
* **Group**: A set of Users that receive the same messages.
* **Group Epoch**: The monotonically increasing reference of the state of a group.
* **Group Master Key**: Used to derive message encryption keys for groups.
* **EncryptedGroupState**: An opaque, end-to-end encrypted representation of a Group State, stored by the server for device synchronization.

## ActivityPub Model

### Users

* Each User is represented as an ActivityPub Actor (typically of type `Person`) with a standard inbox and outbox.

### Devices

* Each User may have one or more Devices. Devices are represented indirectly through published `KeyPackages` in the User.  
* Each Actor exposes a `keyPackage`s collection containing references to `KeyPackage` objects.  
* Device Lifecycle  
  * Add device: the client issues a [Create](https://www.w3.org/TR/activitystreams-vocabulary/#dfn-create) for a `KeyPackage` object, then [Add](https://www.w3.org/TR/activitystreams-vocabulary/#dfn-add)s it the url to the `keyPackages` object.  
  * Remove device: the client issues a [Remove](https://www.w3.org/TR/activitystreams-vocabulary/#dfn-remove) activity for the `KeyPackage`.

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

#### KeyPackage

Example: `KeyPackage` object  
```json  
{  
  "@context": "https://eko.network/ns",  
  "type": "KeyPackage",  
  "id": "https://eko.network/user/user1/keyPackage/A",  
  "deviceId": "<device-id>",  
  "identityKey": "base64-encoded",  
  "registrationId": 1,  
  "preKeyId": 1,  
  "preKey": "base64-encoded",  
  "signedPreKeyId": 1,  
  "signedPreKey": "base64-encoded",  
  "signedPreKeySignature": "base64-encoded"  
}  
```

### Messages

#### SignalEnvelope

All Signal encrypted messages are transported inside a `SignalEnvelope`.

* Targets *one* user.  
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
  "to": ["https://other.network/user/user2"],  
  "object": {  
	"type": ["Object", "SignalEnvelope"],  
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

Groups (see Group Messaging Section for more information):
- GroupCreate
- GroupUpdate
- GroupMemberAdd
- GroupMemberRemove
- GroupKeyRotate
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
  "id": "urn:uuid:<uuid>",  
  "object": {  
	"type": "Note",  
	"id": "urn:uuid:<uuid>",  
	"content": "Hello, World Universe!"  
  }  
}  
```  
Example: Delete activity
```json  
{  
  "@context": "https://www.w3.org/ns/activitystreams",  
  "type": "Delete",  
  "id": "urn:uuid:<uuid>",  
  "object": "urn:uuid:<uuid>"  
}  
```

## Client-to-Server Protocol (C2S)

### Send Message

When sending a message, the client:
1. Fetches each recipient’s `keyPackages` collection.  
2. Encrypts the message for each recipient’s Device using the Signal protocol.  
3. Creates a `SignalEnvelope` containing one encrypted Message per Device.  
4. POSTs a Create activity with the `SignalEvelop` to its outbox.
### Receive Message

#### Message
1. Decrypt message.  
2. Read as ActivityPub.
#### PartialDelivery
1. Re-pull Recipient’s `KeyPackages` and any keys the client does not have.  
2. Resend `SignalEnvelop` with remaining encrypted messages.
#### Reject
1. Re-pull Recipient’s KeyPackages and any keys the client does not have.  
2. Resend `SignalEnvelope` with new encrypted messages.
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
  "to": ["https://eko.network/user/user1"],  
  "object": "https://eko.network/user/signal/<envelope-id>",  
  "summary": "SignalEnvelope rejected: encrypted messages missing for one or more recipient devices."  
}  
```

## Group Messaging
This section defines end-to-end encrypted group messaging using client-managed group state and cryptographic authorization.

Group membership, roles, and state transitions are enforced exclusively by clients.
Servers are intentionally blind to group semantics and MUST NOT interpret, validate, or enforce group state.

This design mirrors Signal's modern group architecture ([paper](https://eprint.iacr.org/2019/1416.pdf)), excluding server-mediated anonymous credential proofs, which are explicitly deferred to future versions of this specification.
### Group
A Group represents an encrypted conversation between multiple members.
- Identified by a stable, `groupId`.  MUST be a 128-bit UUID.
- Exists only within encrypted payloads, client local storage, and encrypted blobs stored on the server.
#### Server Blind Groups
> This version excludes server mediated authorization for group state access or modification. We leave AuthCredentials for a new version of the specification as described in the [blog](https://signal.org/blog/signal-private-group-system/) and [paper](https://eprint.iacr.org/2019/1416.pdf).
- Servers do not authenticate or authorize group membership.
- Servers do not validate group control operations.
- Server store `EncryptedGroupState` objects as opaque blobs without enforcing access control.

All group membership, authorization, and state transition validity are enforced exclusively by clients using shared cryptographic secrets. Anonymous credentials and zero knowledge proofs are out of scope for now.

### Group State
Each client participating in a Group maintains a local Group State that contains the `groupId`, `epoch`, member list, admin roles, and master key.

Example: Group State
```json
{
  "groupId": "urn:uuid:<uuid>",
  "epoch": 5,
  "members": [
    "https://eko.network/user/alice",
    "https://other.network/user/bob"
  ],
  "admins": [
    "https://eko.network/user/alice"
  ],
  "groupMasterKey": "<base64>",
}
```

`epoch`
- Each group maintains a monotonically increasing integer epoch.
- MUST be incremented on any membership or key change.
- Messages referencing a stale or unknown epoch MUST be rejected by the client.

`groupMasterKey`
- Symmetric secret shared by all current group members.
- Derives the message encryption keys and group authentication (signing) keys.
- MUST be rotated whenever a member is removed.

#### Server Encrypted Group State
To support device synchronization and recovery, clients MAY upload encrypted snapshots of Group State to their home server.

Example: Server Visible EncryptedGroupState
```json
{
  "type": "EncryptedGroupState",
  "id": "https://eko.network/user/alice/groupState/<group-id>",
  "groupId": "urn:uuid:<group-id>",
  "epoch": 7,
  "mediaType": "application/eko-group-state",
  "encoding": "base64",
  "content": "<base64-encoded-ciphertext>"
}
```

##### Server Requirements
For each User, the server MUST:
- Store only the latest `EncryptedGroupState` per `groupId`
- Treat all group state blobs as opaque
- Replace older blobs when a higher epoch is received
- Associate blobs only with the owning User

Servers MUST NOT:
- Inspect contents
- Modify contents
- Enforce access control
- Infer membership
#### Group State Authentication
All group control objects MUST be authenticated using a Group Signing Key derived from the previous Group State.

##### Group Signing Key
```
groupSigningKey = HKDF(
  input = groupMasterKey,
  info  = "eko.group.signing",
  length = 32
)
```

##### Authentication Algorithm
Compute the HMAC with the canonical JSON of the group control object.
```
mac = HMAC_SHA256(
    key = groupSigningKey,
    message = canonical_json(groupControlObject)
)
```

Example: Authenticated Group Control Message
```json
{
  "type": "GroupMemberAdd",
  "groupId": "urn:uuid:<group-id>",
  "epoch": 6,
  "added": ["https://new.network/user/charlie"],
  "signature": {
    "alg": "HMAC-SHA256",
    "value": "<base64>"
  }
}
```

To prevent malicious servers from modifying Groups, clients MUST reject unsigned or invalidly signed group control messages.
### Creating a Group
1. Creator generates a `groupId` and `groupMasterKey`.
2. Initializes `epoch=1`, member list, admin list.
3. Sends a `GroupCreate` message to every device of every initial member.

Example: Group Creation
```json
{
  "type": "GroupCreate",
  "groupId": "urn:uuid:<group-id>",
  "epoch": 1,
  "members": [
    "https://eko.network/user/alice",
    "https://other.network/user/bob"
  ],
  "admins": [
    "https://eko.network/user/alice"
  ],
  "groupMasterKey": "<base64>"
}
```

### Modifying Group Membership
Only clients whose local Group State grants them permission (by being present in the admin list) MAY generate valid group control messages.
#### Adding a Member(s)
1. The authorized client
	1. Increments the `epoch`.
	2. Rotates the `groupMasterKey`.
2. Sends
	1. `GroupMemberAdd` to existing members.
	2. `GroupCreate` to new member(s).

Example: Adding a New Group Member
```json
{
  "type": "GroupMemberAdd",
  "groupId": "urn:uuid:<uuid>",
  "epoch": 6,
  "added": [
	"https://new.network/user/charlie"
  ]
}
```
```json
{
  "type": "GroupCreate",
  "groupId": "urn:uuid:<uuid>",
  "epoch": 6,
  "members": [...],
  "groupMasterKey": "<key>"
}
```

#### Removing a Member
1. The authorized client
	1. Removes the member.
	2. Rotates the `groupMasterKey`.
	3. Increments the `epoch`.
2. Sends a `GroupMemberRemove` activity to the remaining members.

Example: Removing a Member
```json
{
  "type": "GroupMemberRemove",
  "id": "urn:uuid:<uuid>",
  "groupId": "urn:uuid:<group-id>",
  "epoch": 7,
  "removed": [
    "https://other.network/user/bob"
  ]
}
```

Note: Because of federation and servers not knowing group membership, removed users may still receive group messages, but cannot decrypt new messages.
### Sending a Group Message
1. Client checks it has the current Group State.
2. Client creates a standard ActivityPub object.
	1. Object includes `groupId` and `epoch`.
3. Object is encrypted using keys derived from `groupMasterKey`.
4. One `SignalEnvelope` is sent per recipient user, containing messages for each device.

Clients MUST drop:
- Messages with stale epochs.
- Messages failing decryption.
- Messages failing group authentication.

Example: Create Note Activity in a Group
```json
{
  "type": "Create",
  "object": {
    "type": "Note",
    "id": "urn:uuid:<uuid>",
    "content": "hello group",
    "groupId": "urn:uuid:<group-id>",
    "epoch": 5
  }
}
```

## E2E Encryption

- TODO: Signal encryption mechanism described here.

## Key Management

- TODO

## Trust Model and Limitations

* Servers are trusted to maintain the device list and correct keys.
  * See Federated Key Transparency work.
* Servers may store encrypted Group State for the purpose of device synchronization. Servers are not trusted to read, interpret, or modify Group State contents. Compromise of a server MUST NOT reveal group membership, cryptographic keys, or message content.

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

