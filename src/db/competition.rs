use std::io::Cursor;

use enumflags2::BitFlags;
use image::{
    imageops::{overlay, FilterType},
    io::Reader as ImageReader,
    DynamicImage, ImageFormat, Rgba,
};
use imageproc::{drawing::draw_antialiased_line_segment_mut, pixelops::interpolate};
use serenity::all::{ChannelId, MessageId};

macro_rules! make_bingo_variants {
    ($($bingo_name:ident: $bingo_description:expr,)*) => {
        #[enumflags2::bitflags]
        #[repr(u32)]
        #[derive(Debug, Clone, Copy, poise::macros::ChoiceParameter, strum::FromRepr)]
        pub enum BingoSquare {
            $(
                #[name = $bingo_description]
                $bingo_name,
            )*
        }
    };
}

make_bingo_variants! {
    Drama: "drama",
    RawManpower: "raw manpower",
    HighDowntime: "high downtime",
    DelayedPrizes: "delayed prizes",
    NoSource: "no source",
    CloudBruteForce: "cloud brute force",
    LowRating: "low rating",
    AdminsAsleep: "admins asleep",
    TooMuchOsing: "too much osint",
    AuthorsTool: "authors tool",
    ForgotFiles: "forgot files",
    FakeFlag: "fake flag",
    Free: "free",
    Guessing: "guessing",
    StolenChallenges: "stolen challenges",
    RetractedAfterSolve: "retracted after solve",
    Stego: "stego",
    ClosedRegistration: "closed registration",
    BrokenRev: "broken rev",
    FrozenScoreboard: "frozen scoreboard",
    HintsAfterSolve: "hints after solve",
    BlindPwn: "blind pwn",
    NoFlagFormat: "no flag format",
    LeakedFlags: "leaked flags",
    InfraHacked: "infra hacked",
}

impl BingoSquare {
    fn from_coords(x: u32, y: u32) -> Option<BingoSquare> {
        // each bingo square has the next bit set because they are a bitmask
        let bits = 1 << (5 * y + x);

        Self::from_repr(bits)
    }
}

// These have to be separate because apparently you can only have 25 choices in a discord choices argument
/*TwitterDrama: "bonus: twitter drama",
CyberLeague: "bonus: cyber league",
AdminsBan: "bonus: admins ban over criticism",*/

#[derive(Debug, Clone)]
pub struct CompetitionRaw {
    // Channel id has to be i64 because sqlite does not support u64?
    pub channel_id: i64,
    pub name: String,
    pub bingo: i64,
    pub active: i64,
    pub ai_allowed: i64,
    pub url: String,
    pub creds_channel_id: i64,
    pub creds_message_id: i64,
    pub username: Option<String>,
    pub password: Option<String>,
    pub email: Option<String>,
    pub token: Option<String>,
}

impl From<Competition> for CompetitionRaw {
    fn from(value: Competition) -> Self {
        CompetitionRaw {
            channel_id: value.channel_id.get() as i64,
            name: value.name,
            bingo: value.bingo.bits().into(),
            active: value.active as i64,
            ai_allowed: value.ai_allowed as i64,
            url: value.url,
            creds_channel_id: value.creds_channel_id.get() as i64,
            creds_message_id: value.creds_message_id.get() as i64,
            username: value.username,
            password: value.password,
            email: value.email,
            token: value.token,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Competition {
    pub channel_id: ChannelId,
    pub name: String,
    pub bingo: BitFlags<BingoSquare>,
    pub active: bool,
    pub ai_allowed: bool,
    pub url: String,
    pub creds_channel_id: ChannelId,
    pub creds_message_id: MessageId,
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
}

impl Competition {
    /// Returns a list of login fields which have been set.
    pub fn login_fields<'a>(&'a self) -> Vec<(&'static str, &'a str)> {
        let mut out = Vec::new();

        if let Some(value) = self.username.as_ref() {
            out.push(("Username", value.as_str()));
        }

        if let Some(value) = self.email.as_ref() {
            out.push(("Email", value.as_str()));
        }

        if let Some(value) = self.password.as_ref() {
            out.push(("Password", value.as_str()));
        }

        if let Some(value) = self.token.as_ref() {
            out.push(("Token", value.as_str()));
        }

        out
    }
}

impl From<CompetitionRaw> for Competition {
    fn from(value: CompetitionRaw) -> Self {
        let creds_channel_id = if value.creds_channel_id == -1 {
            ChannelId::default()
        } else {
            ChannelId::new(value.creds_channel_id as u64)
        };

        let creds_message_id = if value.creds_message_id == -1 {
            MessageId::default()
        } else {
            MessageId::new(value.creds_message_id as u64)
        };

        Competition {
            channel_id: ChannelId::new(value.channel_id as u64),
            name: value.name,
            bingo: BitFlags::from_bits_truncate(value.bingo as u32),
            active: value.active != 0,
            ai_allowed: value.ai_allowed != 0,
            url: value.url,
            creds_channel_id: creds_channel_id,
            creds_message_id: creds_message_id,
            username: value.username,
            password: value.password,
            email: value.email,
            token: value.token,
        }
    }
}

const BINGO_IMAGE: &[u8] = include_bytes!("../../badctf_bingo.png");
const BINGO_X: &[u8] = include_bytes!("../../red_x.png");

const X_SIZE: (u32, u32) = (80, 60);

fn bingo_coord_to_image_coord(x: u32, y: u32) -> (u32, u32) {
    let new_x = 90 * x + 50;
    let new_y = 97 * y + 175;

    (new_x, new_y)
}

fn draw_bingo_line(bingo_image: &mut DynamicImage, start: (u32, u32), end: (u32, u32)) {
    let (start_x, start_y) = bingo_coord_to_image_coord(start.0, start.1);
    let (end_x, end_y) = bingo_coord_to_image_coord(end.0, end.1);

    draw_antialiased_line_segment_mut(
        bingo_image,
        (start_x as i32 - 10, start_y as i32),
        (end_x as i32 - 10, end_y as i32),
        Rgba([255, 0, 0, 255]),
        interpolate,
    );
}

#[derive(Default)]
struct BingoChecker {
    x_count: [u8; 5],
    y_count: [u8; 5],
    positive_diag: u8,
    negative_diag: u8,
}

impl BingoChecker {
    fn mark(&mut self, x: u32, y: u32) {
        self.x_count[x as usize] += 1;
        self.y_count[y as usize] += 1;

        if x == y {
            self.positive_diag += 1;
        }

        if x == 4 - y {
            self.negative_diag += 1;
        }
    }

    /// Draws any winning lines in bingo
    fn check_and_draw_win(&self, image: &mut DynamicImage) {
        for i in 0..5 {
            if self.x_count[i as usize] == 5 {
                draw_bingo_line(image, (i, 0), (i, 4));
            }

            if self.y_count[i as usize] == 5 {
                draw_bingo_line(image, (0, i), (4, i));
            }
        }

        if self.positive_diag == 5 {
            draw_bingo_line(image, (0, 0), (4, 4));
        }

        if self.negative_diag == 5 {
            draw_bingo_line(image, (4, 0), (0, 4));
        }
    }
}

impl Competition {
    pub fn get_bingo_picture(&self) -> Result<DynamicImage, anyhow::Error> {
        let mut bingo_squares = ImageReader::new(Cursor::new(BINGO_IMAGE))
            .with_guessed_format()?
            .decode()?;

        let red_x = ImageReader::new(Cursor::new(BINGO_X))
            .with_guessed_format()?
            .decode()?
            .resize(X_SIZE.0, X_SIZE.1, FilterType::Gaussian);

        let mut solve_checker = BingoChecker::default();

        for x in 0..5 {
            for y in 0..5 {
                let Some(square) = BingoSquare::from_coords(x, y) else {
                    continue;
                };

                if self.bingo.contains(square) {
                    solve_checker.mark(x, y);

                    let (x_pos, y_pos) = bingo_coord_to_image_coord(x, y);

                    overlay(
                        &mut bingo_squares,
                        &red_x,
                        (x_pos - (X_SIZE.0 / 2)) as i64,
                        (y_pos - (X_SIZE.1) / 2) as i64,
                    );
                }
            }
        }

        solve_checker.check_and_draw_win(&mut bingo_squares);

        Ok(bingo_squares)
    }

    pub fn get_bingo_picture_png_bytes(&self) -> Result<Vec<u8>, anyhow::Error> {
        let image = self.get_bingo_picture()?;

        let mut out = Vec::new();
        image.write_to(&mut Cursor::new(&mut out), ImageFormat::Png)?;

        Ok(out)
    }
}
