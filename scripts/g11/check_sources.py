"""Check cached primary PDFs against the curated version/page manifest; no text export."""
import argparse
import hashlib
import json
from pathlib import Path
from pypdf import PdfReader

parser=argparse.ArgumentParser()
parser.add_argument("--cache",type=Path,required=True)
parser.add_argument("--out",type=Path,required=True)
args=parser.parse_args()
root=Path(__file__).resolve().parents[2]
data=json.loads((root/"experiments/g11/literature/evidence.json").read_text())
names={"helios":"helios.pdf","belenios":"belenios-complete.pdf","ordinos":"ordinos-complete.pdf","kryvos":"kryvos-complete.pdf"}
rows=[]
for system in data["systems"]:
    if system["id"] not in names:
        continue
    path=args.cache/names[system["id"]]
    digest=hashlib.sha256(path.read_bytes()).hexdigest()
    if digest!=system["source_sha256"]:
        raise ValueError(f"Wrong source version: {system['id']}")
    reader=PdfReader(path)
    if len(reader.pages)!=system["pdf_pages"]:
        raise ValueError("Wrong page count")
    for index in system["reviewed_pdf_pages"]:
        if len(reader.pages[index-1].extract_text() or "")<100:
            raise ValueError("Listed review page has no extractable text")
    rows.append({"system":system["id"],"source_url":system["url"],"sha256":digest,"pdf_pages":len(reader.pages),
                 "reviewed_pages":system["reviewed_pdf_pages"],"text_extractability_check":"passed"})
result={"status":"PASS_FOUR_PRIMARY_PDF_VERSIONS_AND_PAGE_MAPS","sources":rows,"emami_full_text":"not_obtained", "third_party_full_text_archived":False}
with args.out.open("x") as stream:
    stream.write(json.dumps(result,indent=2)+"\n")
print(json.dumps({"status":result["status"],"source_count":len(rows),"full_text_copied_to_repository":False}))
