ALTER TABLE competition ADD COLUMN active INT NOT NULL DEFAULT 0;

CREATE TABLE active_ctf_members (
    -- Channel ID of CTF
    channel_id INT NOT NULL,
    -- Discord ID of member who joined
    member_id INT NOT NULL,
    join_time TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(channel_id, member_id)
);
