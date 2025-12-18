# Eko-messenger

Version: 0.0.1

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
  "id": "https://eko.network/user/signal/<envelope-id>" // this link should be **empty**  
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
* Article  
* Image  
* Audio  
* Video

### Constraints

The following restrictions apply to content objects embedded in encrypted messages:

* `id`  
  * MUST be a unique, non-resolvable URI.  
  * HTTPS URLs MUST NOT be used.  
  * Client-generated URNs are RECOMMENDED.  
* `mediaType`  
  * For Note and Article, this SHOULD be unset or use the default text/html.  
  * For Image, Audio, or Video (TODO: do we encrypt the same way, and send as activityPub does?)  
* `encoding`  
  * New property, default value is `base64`.  
* `content`  
  * For Note and Article, this MUST contain the HTML content of the object.  
  * TODO: images and others?  
* `summary`  
  * Optional human-readable summary or description of the content.  
* `attachment`  
  * TODO  
* `inReplyTo`  
  * References the id of a content object previously delivered to the same conversation.

Example: Create activity  
```json  
{  
  "@context": "https://www.w3.org/ns/activitystreams",  
  "type": "Create",  
  "id": "urn:eko:uuid:<uuid>",  
  "object": {  
	"type": "Note",  
	"id": "urn:eko:uuid:<uuid>",  
	"content": "Hello, World\!"  
  }  
}  
```  
Example: Update activity  
```json  
{  
  "@context": "https://www.w3.org/ns/activitystreams",  
  "type": "Update",  
  "id": "urn:eko:uuid:<uid>",  
  "object": {  
	"type": "Note",  
	"id": "urn:eko:uuid:<uid>",  
	"content": "Hello, World Universe!"  
  }  
}  
```  
Example: Delete activity  
```json  
{  
  "@context": "https://www.w3.org/ns/activitystreams",  
  "type": "Delete",  
  "id": "urn:eko:uuid:<uid>",  
  "object": "urn:eko:uuid:<uid>"  
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
1. Server MUST notify the Client if the server fails to deliver the messages.

### Receive Message

When a server receives a `SignalEnvelope` in the User’s inbox, it MUST:

1. Verify the envelope contains exactly one encrypted Message for each currently registered Device of the recipient User.  
1. If verification succeeds, the message is put in the User’s inbox, and ACKs the delivery, and a Confirm is (optionally) sent to the client (if the server implementation wants to distinguish a receive to the home server vs external server).  
1. If verification fails, delivery is rejected.  
   1. If Partial Delivery is implemented, the server will put the Messages in the User’s inbox and put a PartialDelivery activity in the Sender’s inbox. MUST wait for the  `SignalEnvelope` with remaining messages.  
   1. Else, a Reject activity is put in the Sender’s inbox.

Example: Partial Delivery  
```json  
{  
  "@context": [  
	"https://www.w3.org/ns/activitystreams",  
	"https://eko.network/ns"  
  ],  
  "type": "eko:PartialDelivery",  
  "actor": "https://other.network",  
  "to": ["https://eko.network/user/user1"],  
  "object": "https://eko.network/user/signal/<envelope-id>",  
  "eko:deviceSetOutOfDate": true,  
  "summary": "SignalEnvelope delivered, but one or more recipient devices were missing encrypted messages."  
}  
```

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
  "eko:deviceSetOutOfDate": true,  
  "eko:partialDelivery": true,
  "summary": "SignalEnvelope rejected: encrypted messages missing for one or more recipient devices."  
}  
```

## E2E Encryption

- TODO: Signal encryption mechanism described here.

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

* Will be implemented, but is handled out-of-band and are not part of the protocol.
