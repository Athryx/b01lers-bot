-- all different info we might wanna display for competition, to support editing
-- also add ai_allowed column

ALTER TABLE competition ADD COLUMN ai_allowed INTEGER NOT NULL DEFAULT 1;
ALTER TABLE competition ADD COLUMN url TEXT NOT NULL DEFAULT '';

-- default to 0 for existing competitions
ALTER TABLE competition ADD COLUMN creds_channel_id INTEGER NOT NULL DEFAULT -1;
ALTER TABLE competition ADD COLUMN creds_message_id INTEGER NOT NULL DEFAULT -1;

ALTER TABLE competition ADD COLUMN username TEXT;
ALTER TABLE competition ADD COLUMN email TEXT;
ALTER TABLE competition ADD COLUMN password TEXT;
ALTER TABLE competition ADD COLUMN token TEXT;
