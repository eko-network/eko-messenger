# WIP: Eko Messages
A standalone, end-to-end encrypted (E2EE) messaging application.

This repository contains the backend server for Eko Messenger, a standalone, end-to-end encrypted (E2EE) messaging application that uses the Eko Social app for authentication.

The server is written in Rust and uses the ActivityPub protocol for federation. While this server powers the Eko Messenger application, it also serves as a reference implementation for a secure, E2EE messaging protocol over ActivityPub.

# [Specification](https://github.com/eko-network/eko-messenger/blob/main/README.md)
The specification is found [here](https://github.com/eko-network/eko-messenger/blob/main/README.md)

# Example

## Sending a message
```mermaid
sequenceDiagram
    autonumber
    participant Alice as Alice Device
    participant ServerA as Server A<br/>(alice@serverA)
    participant ServerB as Server B<br/>(bob@serverB)
    participant BobInbox as Bob User Inbox
    participant Device1 as Bob Device A
    participant Device2 as Bob Device B

    Note over Alice,ServerB: Phase 1 — Device & Key Discovery

    Alice->>ServerA: Fetch keyPackages for bob@serverB
    ServerA->>ServerB: GET /user/bob/keyPackages
    ServerB-->>ServerA: keyPackage refs + public keys
    ServerA-->>Alice: keyPackages collection

    Note over Alice: Phase 2 — Client-side Encryption

    Alice->>Alice: Encrypt ActivityPub activity<br/>for Device A (Signal)
    Alice->>Alice: Encrypt ActivityPub activity<br/>for Device B (Signal)
    Alice->>Alice: Build SignalEnvelope<br/>(one message per device)

    Note over Alice,ServerA: Phase 3 — Client → Server (C2S)

    Alice->>ServerA: POST Create(SignalEnvelope)<br/>to outbox
    ServerA->>ServerA: Validate structure (not content)

    Note over ServerA,ServerB: Phase 4 — Federation (S2S)

    ServerA->>ServerB: POST Create(SignalEnvelope)<br/>to Bob inbox
    ServerB->>ServerB: Verify encrypted message exists<br/>for every registered device

    alt All devices covered
        ServerB-->>ServerA: 202 Accepted
        ServerB->>BobInbox: Enqueue SignalEnvelope
    else Missing or stale devices
        ServerB-->>ServerA: Reject / PartialDelivery
    end

    Note over ServerB: Phase 5 — Device Fanout (Home Server)

    ServerB->>Device1: Deliver encrypted message
    ServerB->>Device2: Deliver encrypted message
    ServerB->>ServerB: Delete envelope after delivery
```

## SignalEnvelope Lifecycle
```mermaid
sequenceDiagram
    participant Sender
    participant SenderServer
    participant ReceiverServer
    participant ReceiverDevice

    Sender->>Sender: Encrypt ActivityPub activity
    Sender->>Sender: Build SignalEnvelope
    Sender->>SenderServer: POST Create(SignalEnvelope)

    SenderServer->>ReceiverServer: POST Create(SignalEnvelope)
    ReceiverServer->>ReceiverServer: Verify device coverage
    ReceiverServer->>ReceiverDevice: Deliver encrypted message
    ReceiverServer->>ReceiverServer: Delete SignalEnvelope
```

## Partial Delivery / Reject
```mermaid
sequenceDiagram
    autonumber
    participant Alice
    participant ServerA
    participant ServerB

    Alice->>ServerA: POST Create(SignalEnvelope)
    ServerA->>ServerB: POST Create(SignalEnvelope)

    alt Missing some devices
        ServerB-->>ServerA: PartialDelivery<br/>+ missingKeyPackages
        ServerA-->>Alice: PartialDelivery
        Alice->>Alice: Fetch missing KeyPackages
        Alice->>Alice: Re-encrypt + retry
    else Stale device set
        ServerB-->>ServerA: Reject<br/>deviceSetOutOfDate
        ServerA-->>Alice: Reject
        Alice->>ServerB: Re-fetch keyPackages collection
        Alice->>Alice: Re-encrypt from scratch
    end
```







```mermaid
sequenceDiagram
    participant Alice as Alice's App
    participant ServerA as Server A<br/>(alice@serverA)
    participant ServerB as Server B<br/>(bob@serverB)
    participant Device1 as Bob Device 1<br/>Queue
    participant Device2 as Bob Device 2<br/>Queue

    Note over Alice,Device2: Step A: Key Discovery (Federated)
    Alice->>ServerA: 1. Get keys for bob@serverB
    ServerA->>ServerB: 2. Request Public Identity Keys<br/>and PreKeys for Bob's devices
    ServerB-->>ServerA: 3. Return keys for<br/>Bob_Device_1 & Bob_Device_2
    ServerA-->>Alice: 4. Pass keys to Alice's App

    Note over Alice,Device2: Step B: Client-Side Encryption
    Alice->>Alice: 1. Encrypt message for Bob_Device_1
    Alice->>Alice: 2. Encrypt message for Bob_Device_2
    Alice->>ServerA: 3. Send combined payload<br/>(both encrypted blobs)

    Note over Alice,Device2: Step C: Federation (First Fan-Out)
    ServerA->>ServerA: Wrap in ActivityPub JSON<br/>(Create activity)
    ServerA->>ServerB: POST to Bob's User Inbox<br/>on Server B

    Note over Alice,Device2: Step D: Device Delivery (Second Fan-Out)
    ServerB->>ServerB: 1. Authenticate request<br/>from serverA
    ServerB->>ServerB: 2. Extract encrypted chunks<br/>for each device ID
    ServerB->>Device1: 3. Drop chunk into<br/>Queue_Bob_Device_1
    ServerB->>Device2: 4. Drop chunk into<br/>Queue_Bob_Device_2
```
## Signal Protocol
```mermaid
sequenceDiagram
    participant Alice
    participant Server
    participant Bob
    
    Note over Alice,Bob: Registration Phase
    
    Alice->>Alice: Generate Identity Key Pair (IKa)
    Alice->>Alice: Generate Signed PreKey Pair (SPKa)
    Alice->>Alice: Generate One-Time PreKeys (OPKa1...OPKn)
    Alice->>Server: Register: IKa_pub, SPKa_pub, OPKa_pub[]
    
    Bob->>Bob: Generate Identity Key Pair (IKb)
    Bob->>Bob: Generate Signed PreKey Pair (SPKb)
    Bob->>Bob: Generate One-Time PreKeys (OPKb1...OPKn)
    Bob->>Server: Register: IKb_pub, SPKb_pub, OPKb_pub[]
    
    Note over Alice,Bob: Alice Initiates Conversation with Bob
    
    Alice->>Server: Request Bob's PreKey Bundle
    Server->>Alice: IKb_pub, SPKb_pub, OPKb1_pub
    Server->>Server: Mark OPKb1 as used
    
    Alice->>Alice: Generate Ephemeral Key Pair (EKa)
    Alice->>Alice: Compute X3DH:<br/>DH1 = DH(IKa, SPKb)<br/>DH2 = DH(EKa, IKb)<br/>DH3 = DH(EKa, SPKb)<br/>DH4 = DH(EKa, OPKb1)
    Alice->>Alice: SK = KDF(DH1 || DH2 || DH3 || DH4)
    Alice->>Alice: Initialize Double Ratchet with SK
    Alice->>Alice: Generate Root Key (RK) and Chain Key (CK)
    
    Alice->>Alice: Encrypt Message M1 with Message Key
    Alice->>Alice: Derive Message Key from Chain Key
    Alice->>Server: Initial Message: IKa_pub, EKa_pub, OPKb1_id, Encrypted(M1)
    Alice->>Bob: Initial Message: IKa_pub, EKa_pub, OPKb1_id, Encrypted(M1)
    
    Note over Bob: Bob Receives First Message
    
    Bob->>Bob: Compute X3DH:<br/>DH1 = DH(SPKb, IKa)<br/>DH2 = DH(IKb, EKa)<br/>DH3 = DH(SPKb, EKa)<br/>DH4 = DH(OPKb1, EKa)
    Bob->>Bob: SK = KDF(DH1 || DH2 || DH3 || DH4)
    Bob->>Bob: Initialize Double Ratchet with SK
    Bob->>Bob: Derive Message Key and Decrypt M1
    Bob->>Bob: Delete OPKb1 private key
    
    Note over Alice,Bob: Double Ratchet in Action
    
    Bob->>Bob: Generate new DH Ratchet Key Pair (DHb)
    Bob->>Bob: DH Ratchet: RK, CKs = KDF(RK || DH(DHb, EKa))
    Bob->>Bob: Derive Message Key from CKs
    Bob->>Bob: Encrypt Response M2
    Bob->>Alice: DHb_pub, Encrypted(M2)
    
    Alice->>Alice: Receive DHb_pub (DH Ratchet Step)
    Alice->>Alice: DH Ratchet: RK, CKr = KDF(RK || DH(EKa, DHb))
    Alice->>Alice: Derive Message Key and Decrypt M2
    Alice->>Alice: Generate new DH Ratchet Key Pair (DHa)
    Alice->>Alice: DH Ratchet: RK, CKs = KDF(RK || DH(DHa, DHb))
    Alice->>Alice: Derive Message Key from CKs
    Alice->>Alice: Encrypt Message M3
    Alice->>Bob: DHa_pub, Encrypted(M3)
    
    Bob->>Bob: Receive DHa_pub (DH Ratchet Step)
    Bob->>Bob: DH Ratchet: RK, CKr = KDF(RK || DH(DHb, DHa))
    Bob->>Bob: Derive Message Key and Decrypt M3
```
