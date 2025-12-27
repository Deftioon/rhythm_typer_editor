use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatmapNote {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startTime: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endTime: Option<u32>,
    #[serde(rename = "type", alias = "note_type")]
    pub note_type: String, // "tap" or "hold"
    pub hitsound: Hitsound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hitsound {
    pub sampleSet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sounds: Option<SoundConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<HitsoundPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hold: Option<HoldConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<HitsoundPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundConfig {
    pub hitnormal: bool,
    pub hitclap: bool,
    pub hitwhistle: bool,
    pub hitfinish: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HitsoundPart {
    pub volume: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sounds: Option<SoundConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldConfig {
    pub volume: u32,
    #[serde(rename = "loop")]
    pub loop_field: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Beatmap {
    pub name: String,
    pub overallDifficulty: f32,
    pub bgFile: String,
    pub notes: Vec<BeatmapNote>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Meta {
    #[serde(default)]
    pub songName: String,
    #[serde(default)]
    pub artistName: String,
    #[serde(default)]
    pub mapper: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub explicit: bool,
    #[serde(default)]
    pub audioFile: String,
    #[serde(default)]
    pub backgroundFiles: Vec<String>,
    #[serde(default)]
    pub videoFile: Option<String>,
    #[serde(default)]
    pub videoStartTime: i64,
    #[serde(default)]
    pub timingPoints: Vec<TimingPoint>,
    #[serde(default)]
    pub bpm: f64,
    #[serde(default)]
    pub offset: i64,
    #[serde(default)]
    pub previewTime: i64,
    #[serde(default)]
    pub difficulties: Vec<MetaDifficulty>,
    #[serde(default)]
    pub hasCustomHitsounds: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingPoint {
    pub id: f64,
    pub time: f64,
    pub bpm: f64,
    pub offset: i64,
    pub timeSignature: [i64; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaDifficulty {
    pub name: String,
    pub filename: String,
}

impl Beatmap {
    pub fn new() -> Self {
        Beatmap {
            name: String::from("New Beatmap"),
            overallDifficulty: 5.0,
            bgFile: String::from("background.png"),
            notes: Vec::new(),
        }
    }

    pub fn default_hitsound() -> Hitsound {
        Hitsound {
            sampleSet: "normal".to_string(),
            volume: Some(100),
            sounds: Some(SoundConfig {
                hitnormal: true,
                hitclap: false,
                hitwhistle: false,
                hitfinish: false,
            }),
            start: None,
            hold: None,
            end: None,
        }
    }

    pub fn add_tap_note(&mut self, key: String, time: u32) {
        self.notes.push(BeatmapNote {
            key,
            time: Some(time),
            startTime: None,
            endTime: None,
            note_type: "tap".to_string(),
            hitsound: Self::default_hitsound(),
        });
        self.notes.sort_by_key(|n| n.get_start_time());
    }

    pub fn add_hold_note(&mut self, key: String, start_time: u32, end_time: u32) {
        self.notes.push(BeatmapNote {
            key,
            time: None,
            startTime: Some(start_time),
            endTime: Some(end_time),
            note_type: "hold".to_string(),
            hitsound: Hitsound {
                sampleSet: "normal".to_string(),
                volume: None,
                sounds: None,
                start: Some(HitsoundPart {
                    volume: 100,
                    sounds: Some(SoundConfig {
                        hitnormal: true,
                        hitclap: false,
                        hitwhistle: false,
                        hitfinish: false,
                    }),
                }),
                hold: Some(HoldConfig {
                    volume: 70,
                    loop_field: "normal".to_string(),
                }),
                end: Some(HitsoundPart {
                    volume: 0,
                    sounds: Some(SoundConfig {
                        hitnormal: true,
                        hitclap: false,
                        hitwhistle: false,
                        hitfinish: false,
                    }),
                }),
            },
        });
        self.notes.sort_by_key(|n| n.get_start_time());
    }

    pub fn delete_note_at(&mut self, key: &str, time_ms: u32) -> bool {
        let key_lc = key.to_lowercase();

        // Prefer deleting a hold that covers time_ms, otherwise delete a tap exactly at time_ms.
        // If multiple candidates exist, delete the one with the closest start time.
        let mut best_idx: Option<usize> = None;
        let mut best_score: u32 = u32::MAX;

        for (idx, n) in self.notes.iter().enumerate() {
            if n.key.to_lowercase() != key_lc {
                continue;
            }

            if n.note_type == "hold" {
                let s = n.get_start_time();
                let e = n.get_end_time();
                if time_ms >= s && time_ms <= e {
                    let score = time_ms.abs_diff(s);
                    if score < best_score {
                        best_score = score;
                        best_idx = Some(idx);
                    }
                }
            } else {
                let s = n.get_start_time();
                if s == time_ms {
                    // Exact tap match is a strong candidate.
                    let score = 0;
                    if score <= best_score {
                        best_score = score;
                        best_idx = Some(idx);
                    }
                }
            }
        }

        if let Some(idx) = best_idx {
            self.notes.remove(idx);
            true
        } else {
            false
        }
    }
}

impl BeatmapNote {
    pub fn get_start_time(&self) -> u32 {
        self.time.or(self.startTime).unwrap_or(0)
    }

    pub fn get_end_time(&self) -> u32 {
        self.endTime.unwrap_or(self.get_start_time())
    }
}
