use std::{
    env,
    fs::{self, File},
    io::{self, BufWriter, Cursor, Write},
    os::unix::prelude::FileExt,
    path::Path,
};

use csv::{Reader, ReaderBuilder, StringRecord, WriterBuilder};
use mktemp::Temp;

const RECORD_COUNTER_SIZE: u64 = 8;
const RECORD_SIZE: u64 = 4 + 1 + 4;

struct MapRecord {
    rsid: u32,
    chrom: u8,
    pos: u32,
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        panic!(
            "Usage: {} ((index mapfile_out) | (map mapfile_in map_from)) outfile",
            args[0]
        )
    }

    let cmd = args[1].clone();

    if cmd == "index" {
        let input_path = Path::new(&args[2]);
        let mapfile_path = Path::new(&args[3]);
        create_map(&input_path, &mapfile_path)?;
    } else if cmd == "map" {
        let input_path = Path::new(&args[2]);
        let mapfile_path = Path::new(&args[3]);
        let outfile = Path::new(&args[4]);
        map_to_loci(&input_path, &mapfile_path, &outfile)?;
    } else {
        panic!("Unsupported command.")
    }

    Ok(())
}

use byteorder::BigEndian;
use byteorder::ReadBytesExt;

fn map_to_loci<P: AsRef<Path>>(src_tsv: &P, mapfile_path: &P, out_path: &P) -> anyhow::Result<()> {
    let map_rdr = File::open(mapfile_path)?;

    let mut tsv_rdr = ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .from_path(src_tsv)?;

    let mut tsv_wtr = WriterBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .from_path(out_path)?;

    let num_keys_in_map = read_u64_at(&map_rdr, 0)?;
    let max_iters = (num_keys_in_map as f64).log2().ceil() as usize;

    for record in tsv_rdr.records() {
        // we're restarting our binary search for every record
        // there's likely a faster way to do this
        let mut start = 0;
        let mut end = num_keys_in_map - 1;

        let record = record?;
        let mut record_iter = record.iter();
        let rsid = rsid_to_u32(record_iter.next().unwrap())?; // panicing on empty lines is fine with me

        for _ in 0..max_iters {
            if end < start {
                // TODO: handle this
                panic!("{} not found in map", rsid);
            }

            let middle = (end + start) / 2;
            let seek_idx = get_map_seek_index(middle);

            match read_u32_at(&map_rdr, seek_idx)?.cmp(&rsid) {
                std::cmp::Ordering::Less => start = middle + 1,
                std::cmp::Ordering::Greater => end = middle - 1,
                std::cmp::Ordering::Equal => {
                    let chrom = u8_to_chrom(read_u8_at(&map_rdr, seek_idx + 4)?)?;
                    let pos = read_u32_at(&map_rdr, seek_idx + 4 + 1)?;
                    let loci = format!("{}:{}", chrom, pos);
                    let mut new_record = StringRecord::new();
                    new_record.push_field(&loci);
                    for field in record_iter {
                        new_record.push_field(field);
                    }
                    tsv_wtr.write_record(new_record.into_iter())?;
                    break;
                }
            }
        }
    }

    Ok(())
}

fn get_map_seek_index(record_idx: u64) -> u64 {
    RECORD_COUNTER_SIZE + (record_idx * RECORD_SIZE)
}

fn create_map<P: AsRef<Path>>(src_tsv: &P, dst: &P) -> anyhow::Result<()> {
    let mut rdr = ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .from_path(src_tsv)?;

    let num_records = write_map_records(dst, &mut rdr)?;
    prepend_file(&num_records.to_be_bytes(), dst)?;

    Ok(())
}

fn write_map_records<P: AsRef<Path>>(dst: &P, rdr: &mut Reader<File>) -> anyhow::Result<usize> {
    // scope of mapfile
    // we want to make sure mapfile is flushed and dropped before we prepend num_records
    let mut map_wtr = BufWriter::new(File::create(dst)?);

    // runtime check if file is sorted and panic if not
    let mut last_rsid = 0;

    let mut num_records: usize = 0;

    for r in rdr.records() {
        let r = r?;
        let (rsid, chrom, pos) = parse_map_record(r)?;
        write_map_record(&mut map_wtr, rsid, chrom, pos)?;
        num_records += 1;

        if last_rsid > rsid {
            panic!("Make sure source map is sorted.")
        }

        last_rsid = rsid;
    }
    map_wtr.flush()?;

    Ok(num_records)
}

fn parse_map_record(r: StringRecord) -> anyhow::Result<(u32, u8, u32)> {
    let rsid = rsid_to_u32(&r[0])?;
    let mut parts = r[1].split(':');
    let chrom = chrom_to_u8(parts.next().unwrap())?;
    let pos = parts.next().unwrap().parse::<u32>()?;
    Ok((rsid, chrom, pos))
}

fn write_map_record(wtr: &mut impl Write, rsid: u32, chrom: u8, pos: u32) -> anyhow::Result<()> {
    wtr.write_all(&rsid.to_be_bytes())?;
    wtr.write_all(&chrom.to_be_bytes())?;
    wtr.write_all(&pos.to_be_bytes())?;
    Ok(())
}

fn prepend_file<P: AsRef<Path>>(data: &[u8], file_path: &P) -> anyhow::Result<()> {
    // Create a temporary file
    let tmp_path = Temp::new_file()?;
    // Open temp file for writing
    let mut tmp = File::create(&tmp_path)?;
    // Open source file for reading
    let mut src = File::open(file_path)?;
    // Write the data to prepend
    tmp.write_all(data)?;
    // Copy the rest of the source file
    io::copy(&mut src, &mut tmp)?;
    fs::remove_file(file_path)?;
    fs::rename(&tmp_path, file_path)?;
    // Stop the temp file being automatically deleted when the variable
    // is dropped, by releasing it.
    tmp_path.release();
    Ok(())
}

fn rsid_to_u32(rsid: &str) -> anyhow::Result<u32> {
    Ok(rsid.replace("rs", "").parse::<u32>()?)
}

fn chrom_to_u8(chrom: &str) -> anyhow::Result<u8> {
    match chrom {
        "X" => Ok(23),
        "Y" => Ok(24),
        "MT" => Ok(25),
        _ => Ok(chrom.parse::<u8>()?),
    }
}

fn u8_to_chrom(x: u8) -> anyhow::Result<String> {
    Ok(match x {
        1..=22 => format!("{x}"),
        23 => "X".into(),
        24 => "Y".into(),
        25 => "MT".into(),
        _ => panic!("Invalid chrom representation {}", x),
    })
}

fn read_u8_at(rdr: &impl FileExt, offset: u64) -> io::Result<u8> {
    let mut buf = [0u8; 1];
    rdr.read_exact_at(&mut buf, offset)?;
    Cursor::new(buf).read_u8()
}

fn read_u32_at(rdr: &impl FileExt, offset: u64) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    rdr.read_exact_at(&mut buf, offset)?;
    Cursor::new(buf).read_u32::<BigEndian>()
}

fn read_u64_at(rdr: &impl FileExt, offset: u64) -> io::Result<u64> {
    let mut buf = [0u8; 8];
    rdr.read_exact_at(&mut buf, offset)?;
    Cursor::new(buf).read_u64::<BigEndian>()
}
