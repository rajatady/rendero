#!/usr/bin/env python3
"""
Extract human genome annotations from UCSC for the genome browser demo.

Downloads:
  - refGene.txt.gz — RefSeq gene annotations (genes, exons, UTRs)
  - cytoBand.txt.gz — chromosome cytogenetic bands

Outputs:
  data/meta.json     — chromosome info, gene counts, layout metadata
  data/chr_NN.bin    — per-chromosome point cloud files [x, y, w, h, r, g, b, a]

Layout:
  - Each chromosome stacked vertically with gap
  - X axis = genomic position (1 world unit = 1kb)
  - Per chromosome: cytoband track, + strand genes, - strand genes, exon detail
  - Color by feature type
"""

import gzip
import io
import json
import struct
import urllib.request
import numpy as np
from pathlib import Path

OUT_DIR = Path(__file__).parent / "data"
SCALE = 1000  # 1 world unit = 1000 bp (1kb)

# Chromosome order and approximate sizes (hg38, in bp)
CHROMOSOMES = [
    ("chr1", 248956422), ("chr2", 242193529), ("chr3", 198295559),
    ("chr4", 190214555), ("chr5", 181538259), ("chr6", 170805979),
    ("chr7", 159345973), ("chr8", 145138636), ("chr9", 138394717),
    ("chr10", 133797422), ("chr11", 135086622), ("chr12", 133275309),
    ("chr13", 114364328), ("chr14", 107043718), ("chr15", 101991189),
    ("chr16", 90338345), ("chr17", 83257441), ("chr18", 80373285),
    ("chr19", 58617616), ("chr20", 64444167), ("chr21", 46709983),
    ("chr22", 50818468), ("chrX", 156040895), ("chrY", 57227415),
]

# Track heights and offsets within each chromosome block
CYTOBAND_H = 8       # cytoband strip height
GENE_PLUS_Y = 12     # + strand genes offset
GENE_MINUS_Y = 28    # - strand genes offset
EXON_PLUS_Y = 14     # exon detail for + strand
EXON_MINUS_Y = 30    # exon detail for - strand
GENE_H = 4           # gene body height
EXON_H = 6           # exon height (taller than gene body)
CHR_BLOCK_H = 50     # total height per chromosome block
CHR_GAP = 20         # gap between chromosome blocks

# Colors
COL_CYTOBAND = {
    'gneg':    (0.85, 0.85, 0.88, 0.5),  # light
    'gpos25':  (0.65, 0.65, 0.70, 0.6),
    'gpos50':  (0.45, 0.45, 0.52, 0.7),
    'gpos75':  (0.30, 0.30, 0.38, 0.8),
    'gpos100': (0.15, 0.15, 0.22, 0.9),
    'acen':    (0.8, 0.3, 0.2, 0.8),     # centromere = red
    'gvar':    (0.5, 0.5, 0.6, 0.6),
    'stalk':   (0.4, 0.4, 0.5, 0.5),
}
COL_GENE_PLUS  = (0.24, 0.56, 0.95, 0.7)   # blue
COL_GENE_MINUS = (0.16, 0.78, 0.55, 0.7)    # teal
COL_EXON_PLUS  = (0.38, 0.65, 1.0, 0.85)    # bright blue
COL_EXON_MINUS = (0.24, 0.88, 0.65, 0.85)   # bright teal
COL_UTR        = (0.6, 0.5, 0.8, 0.6)       # purple


def download(url):
    print(f"  Downloading {url}...")
    req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
    with urllib.request.urlopen(req) as resp:
        return resp.read()


def parse_cytobands(data):
    """Parse cytoBand.txt → dict of chr → [(start, end, name, stain)]"""
    bands = {}
    for line in data.decode().strip().split('\n'):
        parts = line.split('\t')
        if len(parts) < 5:
            continue
        chrom, start, end, name, stain = parts[0], int(parts[1]), int(parts[2]), parts[3], parts[4]
        if chrom not in bands:
            bands[chrom] = []
        bands[chrom].append((start, end, name, stain))
    return bands


def parse_refgene(data):
    """Parse refGene.txt → dict of chr → list of gene dicts"""
    genes = {}
    for line in data.decode().strip().split('\n'):
        parts = line.split('\t')
        if len(parts) < 16:
            continue
        chrom = parts[2]
        strand = parts[3]
        tx_start = int(parts[4])
        tx_end = int(parts[5])
        cds_start = int(parts[6])
        cds_end = int(parts[7])
        exon_count = int(parts[8])
        exon_starts = [int(x) for x in parts[9].rstrip(',').split(',') if x]
        exon_ends = [int(x) for x in parts[10].rstrip(',').split(',') if x]
        name2 = parts[12]  # gene symbol

        if chrom not in genes:
            genes[chrom] = []
        genes[chrom].append({
            'name': name2,
            'strand': strand,
            'tx_start': tx_start,
            'tx_end': tx_end,
            'cds_start': cds_start,
            'cds_end': cds_end,
            'exon_starts': exon_starts,
            'exon_ends': exon_ends,
        })
    return genes


def main():
    OUT_DIR.mkdir(parents=True, exist_ok=True)

    # Download annotation files
    print("Downloading genome annotations...")
    cyto_gz = download("https://hgdownload.soe.ucsc.edu/goldenPath/hg38/database/cytoBand.txt.gz")
    cyto_data = gzip.decompress(cyto_gz)
    cytobands = parse_cytobands(cyto_data)
    print(f"  Cytobands: {sum(len(v) for v in cytobands.values())} bands")

    refgene_gz = download("https://hgdownload.soe.ucsc.edu/goldenPath/hg38/database/refGene.txt.gz")
    refgene_data = gzip.decompress(refgene_gz)
    all_genes = parse_refgene(refgene_data)
    print(f"  Genes: {sum(len(v) for v in all_genes.values())} transcripts")

    # Deduplicate genes by name+chr (keep longest transcript)
    def dedup_genes(gene_list):
        best = {}
        for g in gene_list:
            key = (g['name'], g['strand'])
            span = g['tx_end'] - g['tx_start']
            if key not in best or span > (best[key]['tx_end'] - best[key]['tx_start']):
                best[key] = g
        return list(best.values())

    meta = {
        "chromosomes": [],
        "total_points": 0,
        "total_genes": 0,
        "total_exons": 0,
        "genome": "GRCh38/hg38",
        "scale": SCALE,
    }

    cursor_y = 0.0
    total_points = 0
    total_genes = 0
    total_exons = 0

    for chr_name, chr_size in CHROMOSOMES:
        print(f"Processing {chr_name} ({chr_size:,} bp)...")
        chr_w = chr_size / SCALE  # world width

        points = []

        # ─── Cytobands ───
        bands = cytobands.get(chr_name, [])
        for start, end, name, stain in bands:
            x = start / SCALE
            w = max((end - start) / SCALE, 0.5)
            r, g, b, a = COL_CYTOBAND.get(stain, COL_CYTOBAND['gneg'])
            points.append([x, cursor_y, w, CYTOBAND_H, r, g, b, a])

        # ─── Genes ───
        chr_genes = dedup_genes(all_genes.get(chr_name, []))
        gene_meta_list = []  # per-gene metadata for click-to-inspect
        for gene in chr_genes:
            x = gene['tx_start'] / SCALE
            w = max((gene['tx_end'] - gene['tx_start']) / SCALE, 0.3)
            is_plus = gene['strand'] == '+'
            gy = cursor_y + (GENE_PLUS_Y if is_plus else GENE_MINUS_Y)
            r, g, b, a = COL_GENE_PLUS if is_plus else COL_GENE_MINUS
            points.append([x, gy, w, GENE_H, r, g, b, a])
            total_genes += 1

            exon_count = len(gene['exon_starts'])

            # ─── Exons ───
            for es, ee in zip(gene['exon_starts'], gene['exon_ends']):
                ex = es / SCALE
                ew = max((ee - es) / SCALE, 0.1)
                ey = cursor_y + (EXON_PLUS_Y if is_plus else EXON_MINUS_Y)
                er, eg, eb, ea = COL_EXON_PLUS if is_plus else COL_EXON_MINUS

                # Color UTR regions differently
                if ee <= gene['cds_start'] or es >= gene['cds_end']:
                    er, eg, eb, ea = COL_UTR

                points.append([ex, ey, ew, EXON_H, er, eg, eb, ea])
                total_exons += 1

            gene_meta_list.append({
                "name": gene['name'],
                "start": gene['tx_start'],
                "end": gene['tx_end'],
                "strand": gene['strand'],
                "cds_start": gene['cds_start'],
                "cds_end": gene['cds_end'],
                "exons": exon_count,
                "x": float(x),
                "w": float(w),
                "y": float(gy),
            })

        # Write per-chromosome gene metadata
        genes_fname = f"{chr_name}_genes.json"
        with open(OUT_DIR / genes_fname, "w") as f:
            json.dump(gene_meta_list, f)

        # Write chromosome binary
        if points:
            arr = np.array(points, dtype=np.float32).ravel()
            n_points = len(points)
            fname = f"{chr_name}.bin"
            arr.tofile(str(OUT_DIR / fname))
            total_points += n_points
            fsize = len(arr) * 4

            meta["chromosomes"].append({
                "name": chr_name,
                "size_bp": chr_size,
                "x": 0,
                "y": float(cursor_y),
                "w": float(chr_w),
                "h": CHR_BLOCK_H,
                "genes": len(chr_genes),
                "points": n_points,
                "file": fname,
                "file_bytes": fsize,
                "genes_file": genes_fname,
            })
            print(f"  {chr_name}: {len(chr_genes)} genes, {n_points:,} points, {fsize/1024:.0f}KB")

        cursor_y += CHR_BLOCK_H + CHR_GAP

    meta["total_points"] = total_points
    meta["total_genes"] = total_genes
    meta["total_exons"] = total_exons
    meta["world_width"] = float(max(c["w"] for c in meta["chromosomes"]))
    meta["world_height"] = float(cursor_y - CHR_GAP)

    json_path = OUT_DIR / "meta.json"
    with open(json_path, "w") as f:
        json.dump(meta, f, indent=2)

    print(f"\nTotal: {total_points:,} points ({total_genes:,} genes, {total_exons:,} exons)")
    print(f"World: {meta['world_width']:.0f} × {meta['world_height']:.0f}")
    print(f"Written to {OUT_DIR}")


if __name__ == "__main__":
    main()
