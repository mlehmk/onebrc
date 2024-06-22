use std::fs;
use std::hash::Hasher;
use std::os::windows::fs::OpenOptionsExt;
use std::time::SystemTime;
use hashbrown::HashTable;
use memmap::Mmap;
use ahash::AHasher;
use windows_sys::Win32::Storage::FileSystem::{ACCESS_READ, FILE_FLAG_SEQUENTIAL_SCAN, FILE_SHARE_READ};

struct Entry<'a> {
    name: &'a [u8],
    hash: u64,
    temp: i64,
}

impl<'a> Entry<'a> {
    fn read(buf: &mut &'a [u8]) -> Self {
        let mut hasher = AHasher::default();
        let mut val;
        let start = &buf[..];
        let mut count = 0usize;

        loop {
            match buf[0] {
                b';' => break,
                c => val = c as u64
            }
            match buf[1] {
                b';' => { hasher.write_u64(val); *buf = &buf[1..]; count += 1; break; }
                c => val |= (c as u64) << (8 * 1)
            }
            match buf[2] {
                b';' => { hasher.write_u64(val); *buf = &buf[2..]; count += 2; break; }
                c => val |= (c as u64) << (8 * 2)
            }
            match buf[3] {
                b';' => { hasher.write_u64(val); *buf = &buf[3..]; count += 3; break; }
                c => val |= (c as u64) << (8 * 3)
            }
            match buf[4] {
                b';' => { hasher.write_u64(val); *buf = &buf[4..]; count += 4; break; }
                c => val |= (c as u64) << (8 * 4)
            }
            match buf[5] {
                b';' => { hasher.write_u64(val); *buf = &buf[5..]; count += 5; break; }
                c => val |= (c as u64) << (8 * 5)
            }
            match buf[6] {
                b';' => { hasher.write_u64(val); *buf = &buf[6..]; count += 6; break; }
                c => val |= (c as u64) << (8 * 6)
            }
            match buf[7] {
                b';' => { hasher.write_u64(val); *buf = &buf[7..]; count += 7; break; }
                c => val |= (c as u64) << (8 * 7)
            }
            hasher.write_u64(val);
            count += 8;
            *buf = &buf[8..];
        }
        let hash = hasher.finish();
        let name = &start[0..count];
        assert_eq!(buf[0], b';');
        let mut sign = 1i64;
        val = 0;
        loop {
            if buf[1] == b'-' {
                sign = -1;
                *buf = &buf[2..];
            } else {
                *buf = &buf[1..];
            }
            assert!(buf.len() > 0);
            match buf[0] {
                b'.' => {
                    val = val * 10 + (buf[1] - b'0') as u64;
                    assert_eq!(buf[2], 10u8);
                    *buf = &buf[3..];
                    break;
                }
                c => val = val * 10 + (c - b'0') as u64
            }
        }
        let temp = val as i64 * sign;
        Self {
            name, hash, temp
        }
    }

    fn name(&self) -> &[u8] {
        self.name
    }

    fn hash(&self) -> u64 {
        self.hash
    }

    fn temp(&self) -> f32 {
        self.temp as f32 * 0.1f32
    }

    fn temp10(&self) -> i64 {
        self.temp
    }
}

#[derive(Clone)]
struct CityInfo {
    name: Box<[u8]>,
    hash: u64,
    sum: i64,
    count: u32,
    min: i64,
    max: i64,
}

impl CityInfo {
    fn new(name: &[u8], hash: u64) -> Self {
        Self { name: Box::from(name), hash, sum: 0, count: 0, min: i32::MAX as i64, max: i32::MIN as i64}
    }

    fn values(&self) -> String {
        let min = self.min as f32 * 0.1f32;
        let mean = (self.sum as f32) / (self.count as f32) * 0.1f32;
        let max = self.max as f32 * 0.1f32;
        format!("={:.1}/{:.1}/{:.1}", min, mean, max)
    }

    fn name(&self) -> &[u8] {
        &self.name
    }

    fn utf8_name(&self) -> &str {
        std::str::from_utf8(&self.name).unwrap()
    }

    fn hash(&self) -> u64 {
        self.hash
    }
}

trait Joiner {
    fn join(self, other: Self) -> Self;
}

impl Joiner for HashTable<CityInfo> {
    fn join(mut self, other: Self) -> Self {
        for entry in other {
            self.entry(entry.hash(), |e| entry.name() == e.name(), |e| e.hash())
                .and_modify(|c| {
                    c.count += entry.count;
                    c.min = c.min.min(entry.min);
                    c.max = c.max.max(entry.max);
                })
                .or_insert(entry);
        }
        self
    }
}

fn main() {
    println!("loading");
    let loading_timestamp = SystemTime::now();
    let file = fs::OpenOptions::new()
        .read(true)
        .access_mode(ACCESS_READ)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_SEQUENTIAL_SCAN)
        .open(r"C:\1brc\measurements2.txt").unwrap();
    let mmap = unsafe { Mmap::map(&file).unwrap() };
    let mut hasher = AHasher::default();

    println!("reading");
    let reading_timestamp = SystemTime::now();
    let mut buf = &mmap[..];
    let mut map: HashTable<CityInfo> = HashTable::default();
    let mut counter = 0usize;

    while buf.len() > 0 {
        counter += 1;

        let read = Entry::read(&mut buf);

        let entry =
            map.entry(read.hash(), |t| t.name() == read.name(), |t| t.hash())
            .or_insert_with(|| CityInfo::new(read.name(), read.hash())).into_mut();

        entry.max = entry.max.max(read.temp);
        entry.min = entry.min.min(read.temp);
        entry.sum += read.temp;
        entry.count += 1;
    }

    println!("prepare output");
    let prepare_timestamp = SystemTime::now();
    let mut keys: Vec<_> = map.iter().collect();
    keys.sort_by_key(|e| e.name());

    for entry in keys {
        println!("{}{}", entry.utf8_name(), entry.values());
    }

    println!("done");
    let done_timestamp = SystemTime::now();

    println!("Loading time: {} secs", reading_timestamp.duration_since(loading_timestamp).unwrap().as_secs_f32());
    println!("Reading time: {} secs", prepare_timestamp.duration_since(reading_timestamp).unwrap().as_secs_f32());
    println!("Output time: {} secs", done_timestamp.duration_since(prepare_timestamp).unwrap().as_secs_f32());

    println!("Dataset rows: {}", counter);
}
