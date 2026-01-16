# General Protocol

## Sending a message
```mermaid
sequenceDiagram
    autonumber
    participant Alice as Alice Device
    participant ServerA as Server A<br/>(alice@serverA)
    participant ServerB as Server B<br/>(bob@serverB)
    participant BobInbox as Bob Inbox
    participant BobDev1 as Bob Device A
    participant BobDev2 as Bob Device B

    Note over Alice,ServerB: Phase 1 — Device & Key Discovery

    Alice->>ServerA: Fetch keyPackages for bob@serverB
    ServerA->>ServerB: GET /user/bob/keyPackages
    ServerB-->>ServerA: KeyPackage refs + public keys
    ServerA-->>Alice: keyPackages collection

    Note over Alice: Phase 2 — Client-side Encryption

    Alice->>Alice: Encrypt ActivityPub activity<br/>for Bob Device A (Signal)
    Alice->>Alice: Encrypt ActivityPub activity<br/>for Bob Device B (Signal)
    Alice->>Alice: Build SignalEnvelope<br/>(exactly one message per device)

    Note over Alice,ServerA: Phase 3 — Client → Server (C2S)

    Alice->>ServerA: POST Create(SignalEnvelope)
    ServerA->>ServerA: Validate structure only<br/>(no decryption)

    Note over ServerA,ServerB: Phase 4 — Federation (S2S)

    ServerA->>ServerB: POST Create(SignalEnvelope)
    ServerB->>ServerB: Verify device coverage<br/>(count + deviceIds)

    alt All devices covered
        ServerB-->>ServerA: 202 Accepted
        ServerB->>BobInbox: Enqueue SignalEnvelope
    else Missing or stale devices
        ServerB-->>ServerA: Reject / PartialDelivery
    end

    Note over ServerB: Phase 5 — Device Fanout

    BobDev1->>BobInbox: GET inbox
    BobDev2->>BobInbox: GET inbox
    ServerB->>BobDev1: Deliver encrypted message
    ServerB->>BobDev2: Deliver encrypted message
    ServerB->>ServerB: Delete SignalEnvelope
```

## SignalEnvelope Lifecycle
```mermaid
sequenceDiagram
    participant SenderDevice
    participant SenderServer
    participant ReceiverServer
    participant ReceiverDevice

    SenderDevice->>SenderDevice: Encrypt full ActivityPub activity
    SenderDevice->>SenderDevice: Build SignalEnvelope<br/>(per recipient user)
    SenderDevice->>SenderServer: POST Create(SignalEnvelope)

    SenderServer->>ReceiverServer: POST Create(SignalEnvelope)
    ReceiverServer->>ReceiverServer: Verify device coverage only
    ReceiverServer->>ReceiverDevice: Deliver encrypted message
    ReceiverServer->>ReceiverServer: Delete SignalEnvelope after delivery
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

# Groups

## Group Creation
```mermaid
sequenceDiagram
    autonumber
    participant AliceDev as Alice Device
    participant ServerA as Server A
    participant ServerB as Server B
    participant BobDev as Bob Device

    Note over AliceDev: Create Group State

    AliceDev->>AliceDev: Generate groupId + groupMasterKey
    AliceDev->>AliceDev: Initialize epoch = 1

    Note over AliceDev: Encrypt GroupCreate per recipient

    AliceDev->>AliceDev: Encrypt GroupCreate<br/>for Alice devices
    AliceDev->>AliceDev: Encrypt GroupCreate<br/>for Bob devices

    AliceDev->>ServerA: POST Create(SignalEnvelope)<br/>to Alice
    ServerA->>ServerB: POST Create(SignalEnvelope)<br/>to Bob

    BobDev->>ServerB: GET inbox
    ServerB->>BobDev: Deliver encrypted GroupCreate
    BobDev->>BobDev: Decrypt + store Group State
```

## Sending a Group Message
```mermaid
sequenceDiagram
    autonumber
    participant AliceDev as Alice Device
    participant ServerA as Server A
    participant ServerB as Server B
    participant BobDev as Bob Device
    participant CarolDev as Carol Device

    Note over AliceDev: Prepare Group Message

    AliceDev->>AliceDev: Verify local Group State (epoch=5)
    AliceDev->>AliceDev: Encrypt Note using groupMasterKey

    Note over AliceDev: Per-user Signal fanout

    AliceDev->>AliceDev: Encrypt for Bob devices
    AliceDev->>AliceDev: Encrypt for Carol devices

    AliceDev->>ServerA: POST Create(SignalEnvelope to Bob)
    AliceDev->>ServerA: POST Create(SignalEnvelope to Carol)

    ServerA->>ServerB: Federate Bob envelope
    ServerA->>ServerC: Federate Carol envelope

    BobDev->>ServerB: GET inbox
    CarolDev->>ServerC: GET inbox

    BobDev->>BobDev: Decrypt message<br/>+ verify group signature
    CarolDev->>CarolDev: Decrypt message<br/>+ verify group signature
```

## Group Member Removal
```mermaid
sequenceDiagram
    autonumber
    participant AliceDev
    participant ServerA
    participant ServerB
    participant BobDev

    Note over AliceDev: Admin removes Bob

    AliceDev->>AliceDev: Remove Bob from members
    AliceDev->>AliceDev: Rotate groupMasterKey
    AliceDev->>AliceDev: Increment epoch

    AliceDev->>AliceDev: Encrypt GroupMemberRemove<br/>for remaining members

    AliceDev->>ServerA: POST Create(SignalEnvelope)
    ServerA->>ServerB: Federate to remaining members

    BobDev->>ServerB: GET inbox
    BobDev->>BobDev: Receive message<br/>but fail decryption (old key)
```

## Encrypted Group State Upload
```mermaid
sequenceDiagram
    participant AliceDevice
    participant ServerA
    participant AliceNewDevice

    AliceDevice->>AliceDevice: Encrypt Group State snapshot
    AliceDevice->>ServerA: PUT EncryptedGroupState<br/>(epoch=7)

    AliceNewDevice->>ServerA: GET EncryptedGroupState
    AliceNewDevice->>AliceNewDevice: Decrypt + restore Group State
```
