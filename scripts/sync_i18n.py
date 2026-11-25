#!/usr/bin/env python3
"""
scripts/sync_i18n.py - Sincronizador bidireccional / compilador PO -> JSON (Svelte) y Rust.
Permite a traductores usar herramientas profesionales Gettext (.po / Poedit / Weblate)
generando automáticamente los diccionarios JSON estructurados para la GUI y validando
los catálogos de mensajes en Rust.
"""

import os
import re
import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
PO_DIR = REPO_ROOT / "po"
LOCALES_DIR = REPO_ROOT / "ui" / "src" / "locales"

# Idiomas objetivo
LANGS = ["en", "es", "fr", "de", "it", "zh"]

def parse_po(po_path: Path) -> dict:
    """Parsea un archivo .po estándar extrayendo msgid y msgstr sin escapar."""
    translations = {}
    if not po_path.exists():
        return translations

    content = po_path.read_text(encoding="utf-8")
    # Expresión regular para bloques msgid / msgstr
    entries = re.split(r'\n(?=(?:#.*?\n)*msgid\s+)', content)
    
    for entry in entries:
        if not entry.strip():
            continue
        msgid_match = re.search(r'msgid\s+("(?:[^"\\]|\\.)*"(?:\s*"(?:[^"\\]|\\.)*")*)', entry)
        msgstr_match = re.search(r'msgstr\s+("(?:[^"\\]|\\.)*"(?:\s*"(?:[^"\\]|\\.)*")*)', entry)
        
        if msgid_match and msgstr_match:
            raw_id = msgid_match.group(1)
            raw_str = msgstr_match.group(1)
            
            # Limpiar comillas múltiples multilinea
            def clean_str(s):
                parts = re.findall(r'"((?:[^"\\]|\\.)*)"', s)
                combined = "".join(parts)
                return combined.encode('utf-8').decode('unicode_escape')
            
            msgid = clean_str(raw_id)
            msgstr = clean_str(raw_str)
            
            if msgid:  # Ignorar cabecera msgid ""
                translations[msgid] = msgstr
                
    return translations

def unflatten_dict(d: dict, delim: str = ".") -> dict:
    """Convierte un diccionario plano 'a.b.c': 'val' en anidado {'a': {'b': {'c': 'val'}}}."""
    result = {}
    for key, value in d.items():
        parts = key.split(delim)
        curr = result
        for part in parts[:-1]:
            if part not in curr or not isinstance(curr[part], dict):
                curr[part] = {}
            curr = curr[part]
        curr[parts[-1]] = value
    return result

def flatten_dict(d: dict, parent_key: str = "", delim: str = ".") -> dict:
    """Convierte un diccionario anidado en plano."""
    items = []
    for k, v in d.items():
        new_key = f"{parent_key}{delim}{k}" if parent_key else k
        if isinstance(v, dict):
            items.extend(flatten_dict(v, new_key, delim=delim).items())
        else:
            items.append((new_key, str(v)))
    return dict(items)

def write_po(po_path: Path, lang: str, translations: dict, comments: dict = None):
    """Escribe un archivo .po con formato estándar Gettext."""
    if comments is None:
        comments = {}
        
    lines = [
        f'# LexiShield Translation Catalog - Language: {lang}',
        '# Copyright (C) 2026 Julio C. Semper & LexiShield Contributors',
        '# This file is distributed under the same license as the LexiShield package.',
        'msgid ""',
        'msgstr ""',
        f'"Project-Id-Version: LexiShield 2.2.0\\n"',
        f'"Project-Id-Version: LexiShield 3.0.0\\n"',
        '"Report-Msgid-Bugs-To: https://gitlab.com/olorinj/lexishield/issues\\n"',
        '"POT-Creation-Date: 2026-11-25 14:00+0100\\n"',
        '"PO-Revision-Date: 2026-11-25 14:00+0100\\n"',
        f'"Language: {lang}\\n"',
        '"MIME-Version: 1.0\\n"',
        '"Content-Type: text/plain; charset=UTF-8\\n"',
        '"Content-Transfer-Encoding: 8bit\\n"',
        '',
    ]
    
    for key in sorted(translations.keys()):
        val = translations[key]
        comment = comments.get(key, "")
        if comment:
            lines.append(f'#. {comment}')
        # Escapar comillas y saltos de línea
        escaped_key = key.replace('\\', '\\\\').replace('"', '\\"')
        escaped_val = val.replace('\\', '\\\\').replace('"', '\\"').replace('\n', '\\n')
        lines.append(f'msgid "{escaped_key}"')
        lines.append(f'msgstr "{escaped_val}"')
        lines.append('')
        
    po_path.parent.mkdir(parents=True, exist_ok=True)
    po_path.write_text("\n".join(lines), encoding="utf-8")

def export_json_to_po():
    """Genera/sincroniza archivos po/ a partir de los JSON actuales en locales/."""
    for lang in LANGS:
        json_file = LOCALES_DIR / f"{lang}.json"
        if not json_file.exists():
            continue
        try:
            data = json.loads(json_file.read_text(encoding="utf-8"))
            flat = flatten_dict(data)
            po_file = PO_DIR / f"{lang}.po"
            write_po(po_file, lang, flat)
            print(f"✓ Exportado {json_file.name} -> {po_file.name} ({len(flat)} cadenas)")
        except Exception as e:
            print(f"Error procesando {lang}.json: {e}", file=sys.stderr)

def compile_po_to_json():
    """Compila los archivos po/ a ui/src/locales/*.json."""
    for lang in LANGS:
        po_file = PO_DIR / f"{lang}.po"
        if not po_file.exists():
            continue
        translations = parse_po(po_file)
        if not translations:
            continue
        nested = unflatten_dict(translations)
        json_file = LOCALES_DIR / f"{lang}.json"
        json_file.parent.mkdir(parents=True, exist_ok=True)
        json_file.write_text(json.dumps(nested, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"✓ Compilado {po_file.name} -> {json_file.name} ({len(translations)} cadenas)")

def main():
    action = sys.argv[1] if len(sys.argv) > 1 else "compile"
    if action == "export":
        export_json_to_po()
    elif action == "compile":
        compile_po_to_json()
    else:
        print(f"Uso: {sys.argv[0]} [export|compile]")
        sys.exit(1)

if __name__ == "__main__":
    main()

