pub fn atom(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
    bytes.extend_from_slice(kind);
    bytes.extend_from_slice(payload);
    bytes
}
pub fn group(kind: &[u8; 4], parts: &[Vec<u8>]) -> Vec<u8> {
    atom(kind, &parts.concat())
}
pub fn table(kind: &[u8; 4], version: u8, rows: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = vec![version, 0, 0, 0];
    bytes.extend_from_slice(&(rows.len() as u32).to_be_bytes());
    for row in rows {
        bytes.extend_from_slice(row);
    }
    atom(kind, &bytes)
}
pub fn words(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_be_bytes()).collect()
}
pub fn clock(kind: &[u8; 4], version: u8, scale: u32, duration: u64) -> Vec<u8> {
    let mut data = vec![0; if version == 0 { 20 } else { 32 }];
    data[0] = version;
    let at = if version == 0 { 12 } else { 20 };
    data[at..at + 4].copy_from_slice(&scale.to_be_bytes());
    if version == 0 {
        data[16..20].copy_from_slice(&(duration as u32).to_be_bytes());
    } else {
        data[24..32].copy_from_slice(&duration.to_be_bytes());
    }
    atom(kind, &data)
}
pub fn edit(version: u8, duration: u64, time: i64, rate: i32) -> Vec<u8> {
    let mut row = if version == 0 {
        [
            (duration as u32).to_be_bytes(),
            (time as i32).to_be_bytes(),
            rate.to_be_bytes(),
        ]
        .concat()
    } else {
        [
            duration.to_be_bytes().as_slice(),
            time.to_be_bytes().as_slice(),
            rate.to_be_bytes().as_slice(),
        ]
        .concat()
    };
    // Keep ownership explicit for call sites composing multiple repeated edits.
    row.shrink_to_fit();
    row
}
pub struct Fixture {
    pub stbl: Vec<Vec<u8>>,
    pub edits: Vec<u8>,
    pub dref: Vec<u8>,
    pub version: u8,
}
impl Default for Fixture {
    fn default() -> Self {
        let entry = [vec![0; 6], vec![0, 1]].concat();
        let stsd = table(b"stsd", 0, &[atom(b"avc1", &entry), atom(b"hvc1", &entry)]);
        let stsz = atom(b"stsz", &words(&[0, 0, 4, 2, 2, 2, 2]));
        Self {
            version: 0,
            stbl: vec![
                stsd,
                stsz,
                table(b"stts", 0, &[words(&[4, 10])]),
                table(
                    b"ctts",
                    0,
                    &[
                        words(&[1, 0]),
                        words(&[1, 20]),
                        words(&[2, (-10i32) as u32]),
                    ],
                ),
                table(b"stsc", 0, &[words(&[1, 2, 1]), words(&[2, 2, 2])]),
                table(b"stco", 0, &[words(&[32]), words(&[36])]),
                table(b"stss", 0, &[words(&[1]), words(&[3])]),
            ],
            edits: table(
                b"elst",
                0,
                &[
                    edit(0, 100, -1, 1 << 16),
                    edit(0, 400, 0, 1 << 16),
                    edit(0, 200, 20, 0),
                ],
            ),
            dref: table(b"dref", 0, &[atom(b"url ", &[0, 0, 0, 1])]),
        }
    }
}
impl Fixture {
    pub fn replace(&mut self, kind: &[u8; 4], value: Vec<u8>) {
        let i = self
            .stbl
            .iter()
            .position(|b| &b[4..8] == kind)
            .expect("fixture table");
        self.stbl[i] = value;
    }
    pub fn movie(&self) -> Vec<u8> {
        let mut tkhd = vec![0; if self.version == 0 { 84 } else { 96 }];
        tkhd[0] = self.version;
        let id_at = if self.version == 0 { 12 } else { 20 };
        tkhd[id_at..id_at + 4].copy_from_slice(&7u32.to_be_bytes());
        let matrix_at = if self.version == 0 { 40 } else { 52 };
        for (i, word) in [65536i32, 0, 0, 0, 65536, 0, 0, 0, 1 << 30]
            .iter()
            .enumerate()
        {
            tkhd[matrix_at + i * 4..matrix_at + i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        let hdlr = atom(b"hdlr", &[vec![0; 8], b"vide".to_vec()].concat());
        group(
            b"moov",
            &[
                clock(b"mvhd", self.version, 1000, 700),
                group(
                    b"trak",
                    &[
                        atom(b"tkhd", &tkhd),
                        group(b"edts", std::slice::from_ref(&self.edits)),
                        group(
                            b"mdia",
                            &[
                                clock(b"mdhd", self.version, 100, 40),
                                hdlr,
                                group(
                                    b"minf",
                                    &[
                                        group(b"dinf", std::slice::from_ref(&self.dref)),
                                        group(b"stbl", &self.stbl),
                                    ],
                                ),
                            ],
                        ),
                    ],
                ),
            ],
        )
    }
    pub fn file(&self) -> Vec<u8> {
        [
            atom(b"ftyp", b"isom\0\0\0\0isomiso2"),
            atom(b"mdat", &[1, 2, 3, 4, 5, 6, 7, 8]),
            self.movie(),
        ]
        .concat()
    }
}
pub fn signed_fixture() -> Fixture {
    let mut f = Fixture::default();
    f.replace(
        b"ctts",
        table(
            b"ctts",
            1,
            &[
                words(&[1, 0]),
                words(&[1, 20]),
                words(&[2, (-10i32) as u32]),
            ],
        ),
    );
    f
}
