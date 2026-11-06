"""Six consistent source-backed architecture pages, no performance claims."""
import argparse
import hashlib
import json
from pathlib import Path
from xml.sax.saxutils import escape
from reportlab.lib import colors
from reportlab.lib.enums import TA_LEFT
from reportlab.lib.styles import ParagraphStyle
from reportlab.pdfgen import canvas
from reportlab.platypus import Paragraph


def paragraph(c, text, x, y, width, size=10, color="#263B54"):
    style = ParagraphStyle("body", fontName="Helvetica", fontSize=size, leading=size * 1.4,
                           textColor=colors.HexColor(color), alignment=TA_LEFT)
    p = Paragraph(escape(text), style)
    _, height = p.wrap(width, 1000)
    p.drawOn(c, x, y - height)
    return y - height


def render(source, target):
    if target.exists():
        raise ValueError("No overwrite of existing PDF")
    data = json.loads(source.read_text())
    target.parent.mkdir(parents=True, exist_ok=True)
    c = canvas.Canvas(str(target), pagesize=(1000, 670), invariant=1)
    c.setTitle("G11 Architecture and Trust Boundaries")
    c.setSubject("evidence_sha256="+hashlib.sha256(source.read_bytes()).hexdigest())
    headings = ["Initialization", "Registration", "Ballot generation", "Validation / accumulation", "Final tally", "Disclosure"]
    for index, system in enumerate(data["systems"], 1):
        paragraph(c, f"{system['name']} | Architecture and trust boundary", 40, 630, 920, 21)
        paragraph(c, system["version"], 40, 594, 920, 11)
        if system["id"] == "emami":
            paragraph(c, "PENDING ORIGINAL FULL TEXT - no inferred security properties or implemented baseline", 40, 563, 920, 11, "#9A5421")
        elif system["id"] == "verivoting":
            paragraph(c, "NATIVE + LOCAL EVM FALLBACK - exact SnarkPack EVM remains incomplete", 40, 563, 920, 11, "#9A5421")
        else:
            paragraph(c, "Selected version only. Public recomputation is a valid consistency check; no state SNARK is required.", 40, 563, 920, 10)
        for i, stage in enumerate(system["architecture"]["stages"]):
            x = 40 + i * 155
            c.setFillColor(colors.HexColor("#F0F3F7"))
            c.roundRect(x, 388, 142, 136, 8, fill=1, stroke=0)
            paragraph(c, f"{i+1}. {headings[i]}", x+10, 510, 122, 10)
            bottom = paragraph(c, stage, x+10, 475, 122, 11)
            if bottom < 399:
                raise ValueError(f"Stage text overflows {system['id']} {i}")
            if i < 5:
                c.setStrokeColor(colors.HexColor("#6D8299"))
                c.line(x+144, 455, x+153, 455)
                c.line(x+153,455,x+149,459)
                c.line(x+153,455,x+149,451)
        y = 360
        for label, key in [("Execution", "execution"), ("Key custody", "private_key_custody"),
                           ("Tally computation", "tally_actor"), ("Full tally visibility", "full_tally_visibility"),
                           ("Public verification", "public_check")]:
            y = paragraph(c, f"{label}: {system['architecture'][key]}", 40, y, 920, 11)-9
        y = paragraph(c, "Trust assumptions: "+system["trust"]["assumptions"],40,y-4,920,11)-12
        if y < 145:
            raise ValueError("Trust block overlaps sources")
        paragraph(c, "Evidence: "+"; ".join(dict.fromkeys(system["architecture"]["stage_locators"])),40,137,920,9)
        paragraph(c, system["url"],40,84,920,8)
        paragraph(c, f"G11-A/B | 2026-11-06 | Page {index}/6 | Literature comparison, not an external performance experiment",40,36,920,9)
        c.showPage()
    c.save()


if __name__ == "__main__":
    parser=argparse.ArgumentParser()
    parser.add_argument("--source",type=Path,required=True)
    parser.add_argument("--out",type=Path,required=True)
    args=parser.parse_args()
    render(args.source,args.out)
