CREATE TABLE active_ctfs (
    -- Message ID of "click to join" message
    join_id INT NOT NULL,
    -- Channel ID of CTF
    channel_id INT NOT NULL,
    -- Name of the ctf
    name TEXT NOT NULL,
    PRIMARY KEY(channel_id)
);


CREATE TABLE active_ctf_members (
    -- Channel ID of CTF
    channel_id INT NOT NULL,
    -- Discord ID of member who joined
    member_id TEXT NOT NULL,
    join_time TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(channel_id, member_id)
);
