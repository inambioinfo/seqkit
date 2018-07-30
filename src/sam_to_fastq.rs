
use common::{parse_args, GzipWriter};
use std::str;
use std::collections::HashMap;
use rust_htslib::bam;
use rust_htslib::bam::Read;
use rust_htslib::bam::record::Record;

const USAGE: &str = "
Usage:
  sam to raw <bam_file> <out_prefix>
  sam to fasta <bam_file> <out_prefix>
  sam to fastq <bam_file> <out_prefix>
";

#[derive(PartialEq, Clone, Copy)]
enum OutputFormat { RAW, FASTA, FASTQ }
use self::OutputFormat::*;

fn sequence(read: &Record, min_baseq: u8) -> String {
	let seq = read.seq();
	let qual = read.qual();
	let mut ret = String::with_capacity(seq.len());
	if read.is_reverse() {
		// Output reverse complement of the sequence stored in BAM file.
		for k in (0..seq.len()).rev() {
			if qual[k] < min_baseq {
				ret.push('N');
			} else {
				ret.push(match seq.encoded_base(k) {
					1 => 'T', 2 => 'G', 4 => 'C', 8 => 'A', _ => 'N'
				});
			}
		}
	} else {
		// Output the sequence as it is stored in the BAM file.
		for k in 0..seq.len() {
			if qual[k] < min_baseq {
				ret.push('N');
			} else {
				ret.push(match seq.encoded_base(k) {
					1 => 'A', 2 => 'C', 4 => 'G', 8 => 'T', _ => 'N'
				});
			}
		}
	}
	ret
}

pub fn main() {
	let args = parse_args(USAGE);
	let bam_path = args.get_str("<bam_file>");
	let out_prefix = args.get_str("<out_prefix>");

	// TODO: Add support for FASTQ format (must add BASEQ to HashMap...)
	// TODO: Consider making this function generic over the output format.
	let output_format = if args.get_bool("raw") { RAW }
		else if args.get_bool("fasta") { FASTA }
		else if args.get_bool("fastq") { FASTQ }
		else { error!("Invalid output format."); };

	let mut bam = if bam_path == "-" {
		bam::Reader::from_stdin().unwrap()
	} else {
		bam::Reader::from_path(&bam_path).unwrap_or_else(
			|_| error!("Cannot open BAM file '{}'", bam_path))
	};

	let extension = match output_format {
		RAW => "seq", FASTA => "fa", FASTQ => "fq"
	};
	let mut out_1 = GzipWriter::new(&format!("{}_1.{}.gz", out_prefix, extension));
	let mut out_2 = GzipWriter::new(&format!("{}_2.{}.gz", out_prefix, extension));
	let mut out_single = GzipWriter::new(&format!("{}.{}.gz", out_prefix, extension));

	let mut reads_1: HashMap<String, Box<str>> = HashMap::new();
	let mut reads_2: HashMap<String, Box<str>> = HashMap::new();

	for r in bam.records() {
		let mut read = r.unwrap_or_else(
			|_| error!("Input BAM file ended prematurely."));
		if read.is_secondary() || read.is_supplementary() { continue; }

		let qname = str::from_utf8(read.qname()).unwrap();
		let mut read_seq = sequence(&read, 10);

		if output_format == FASTQ {
			read_seq.push('|');
			for baseq in read.qual() {
				read_seq.push(char::from(33 + baseq));
			}
		}

		if read.is_paired() == false {
			write!(out_single, ">{}\n{}\n", qname, read_seq);
		} else if read.is_first_in_template() {
			if let Some(mate_seq) = reads_2.remove(qname) {
				write_read(&mut out_1, output_format, qname, &read_seq);
				write_read(&mut out_2, output_format, qname, &mate_seq);
			} else {
				reads_1.insert(qname.into(), read_seq.into());
			}
		} else if read.is_last_in_template() {
			if let Some(mate_seq) = reads_1.remove(qname) {
				write_read(&mut out_1, output_format, qname, &mate_seq);
				write_read(&mut out_2, output_format, qname, &read_seq);
			} else {
				reads_2.insert(qname.into(), read_seq.into());
			}
		}
	}
}

fn write_read(out: &mut GzipWriter, format: OutputFormat, qname: &str,
	seq: &str) {
	if format == FASTQ {
		let seq_len = (seq.len() - 1) / 2;
		write!(out, "@{}\n{}\n+\n{}\n", qname, &seq[0..seq_len], &seq[seq_len+1..]);
	} else if format == FASTA {
		write!(out, ">{}\n{}\n", qname, seq);
	} else if format == RAW {
		write!(out, "{}\n", seq);
	}
}
