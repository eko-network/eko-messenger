# Activity Behavior
## Database
The Database has three tables, activities, message_entries and deliver_requests.

* `activites`: Basic activity information including a JSONB column.
* `message_entries`: stores the inner MessageEntries that are device specific from a signal envelope. This allows deleting data on deliver without modifying JSON. Also enables the use of BYTEA saving ~33% space for encrypted content.
* `deliver_requests`: PRIMARY KEY (did, message_id). Symbolizes a message waiting for a device. Trigger on DELETE removes corresponding message_entry if it exists and activity if not other deliver_request for given message_id exist.

## Activities
All activities get delivered, but Take only gets delivered to the related device.
### Create
#### POST To Outbox
1. INSERT activity entry into table.
2. INSERT message entries into table.
3. INSERT deliver request into table.
4. Try to send over socket else try to send over web push.
#### GET from Inbox
1. Return activity and the message entry corresponding to device.

### Take
#### POST to Outbox
1. Try to send over socket else INSERT into table with deliver request for device.
#### GET from Inbox
1. Return activity, DELETE deliver request.

### Delivered
#### POST to Outbox
1. If Delivered does not point to a Create, ignore.
2. DELETE deliver request for the associated Create for the associated device.
3. Try to send over socket for all devices, if any device fails INSERT activity entry and deliver request.
#### GET from Inbox
1. Return Activity and remove associated deliver request.

### Reject
#### POST to Outbox
1. If Reject does not point to a Create, ignore.
2. INSERT activity entry and a delivery request for each device.
3. Try to send over socket else try to send over web push.
#### GET from Inbox
1. Return Activity.

### Update
#### POST to Outbox
1. If Update does not point to a Create, ignore.
2. Delete requests and activity for corresponding Reject if it exists.
3. Update Create in place. <!-- Not entirely sure if this makes sense, might be better to insert a standalone activity, but at the moment I don't see a need. -->
#### GET from Inbox
1. Never returned since it gets "absorbed" into the Create.

### Confirm
#### POST to Outbox
1. Ignore
#### POST to Inbox (from remote)
1. If Confirm does not point to a Create, ignore.
2. DELETE deliver request for the associated Create.
3. Try to send over socket for all devices, if any device fails INSERT activity entry and deliver request.
#### GET from Inbox
1. Return Activity and remove associated deliver request.

## Remote Delivery

Remote servers naturally don't have device IDs. Any activity sent remote will be sent in full and persisted locally until we can be confident it has been received. This may be after a successful handshake or after some amount of time.
