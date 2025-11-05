-- Add an "active" column to the competition table
CREATE TEMPORARY TABLE competition_backup (
    -- Id of the competition channel
    channel_id INT NOT NULL,
    -- Name of the ctf
    name TEXT NOT NULL,
    -- bitfield specifying which of the bad ctf bingos have been achieved
    bingo INT NOT NULL,
    -- boolean specifying whether the competition is currently active
    active INT NOT NULL,
    PRIMARY KEY(channel_id)
);

INSERT INTO competition_backup SELECT *, 0 FROM competition;

DROP TABLE competition;
CREATE TABLE competition (
    -- Id of the competition channel
    channel_id INT NOT NULL,
    -- Name of the ctf
    name TEXT NOT NULL,
    -- bitfield specifying which of the bad ctf bingos have been achieved
    bingo INT NOT NULL,
    -- boolean specifying whether the competition is currently active
    active INT NOT NULL,
    PRIMARY KEY(channel_id)
);

INSERT INTO competition SELECT * from competition_backup;

CREATE TABLE active_ctf_members (
    -- Channel ID of CTF
    channel_id INT NOT NULL,
    -- Discord ID of member who joined
    member_id TEXT NOT NULL,
    join_time TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(channel_id, member_id)
);
