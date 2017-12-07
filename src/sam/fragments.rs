
use parse_args;
use std::str;
use rust_htslib::bam;
use rust_htslib::bam::Read;

const USAGE: &'static str = "
Usage:
  sam fragments [options] <bam_file>

Options:
  --min-size=N     Minimum fragment size [default: 0]
  --max-size=N     Maximum fragment size [default: 5000]
";

pub fn main() {
	let args = parse_args(USAGE);
	let bam_path = args.get_str("<bam_file>");
	let min_frag_size: i32 = args.get_str("--min-size").parse().unwrap();
	let max_frag_size: i32 = args.get_str("--max-size").parse().unwrap();

	let mut bam = bam::Reader::from_path(&bam_path).unwrap();

	let mut chr_names: Vec<String> = Vec::new();
	for name in bam.header().target_names() {
		chr_names.push(str::from_utf8(name).unwrap().to_string());
	}

	for r in bam.records() {
		let read = r.unwrap();
		if read.is_paired() == false { continue; }
		if read.is_unmapped() || read.is_mate_unmapped() { continue; }
		if read.is_duplicate() || read.is_secondary() { continue; }
		if read.is_supplementary() { continue; }
		if read.tid() != read.mtid() { continue; }

		if read.pos() > read.mpos() || (read.pos() == read.mpos() && !read.is_first_in_template()) {
			continue;
		}

		let frag_size = read.insert_size().abs();
		if frag_size > max_frag_size || frag_size < min_frag_size { continue; }

		// Output in BED format (0-based half-open segments)
		println!("{}\t{}\t{}", chr_names[read.tid() as usize], read.pos(), read.pos() + frag_size);
    }
}
