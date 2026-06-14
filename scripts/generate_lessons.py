#!/usr/bin/env python3
"""
Generate assets/lessons.json from the Lichess open puzzle database.
Usage: python3 scripts/generate_lessons.py [target_count]

Requires: pip install chess zstandard
Source:   https://database.lichess.org/ (CC0 license)

Lichess CSV format:
  PuzzleId, FEN, Moves, Rating, RatingDeviation, Popularity, NbPlays, Themes, GameUrl, OpeningTags
  - FEN: position BEFORE the opponent's setup move (opponent's turn)
  - Moves[0]: opponent's blunder/setup move → apply to get puzzle position
  - Moves[1]: player's winning answer
  - Moves[2+]: continuation for multi-move puzzles
"""

import sys
import os
import json
import io
import urllib.request

LICHESS_URL = "https://database.lichess.org/lichess_db_puzzle.csv.zst"
OUTPUT_PATH = os.path.join(os.path.dirname(__file__), "..", "assets", "lessons.json")
OUTPUT_PATH = os.path.normpath(OUTPUT_PATH)

# Theme priorities: first match wins for the title
THEME_TITLES = [
    ("mateIn1",          "Mate en 1"),
    ("mateIn2",          "Mate en 2"),
    ("mateIn3",          "Mate en 3"),
    ("backRankMate",     "Mate de Pasillo"),
    ("smotheredMate",    "Mate Ahogado"),
    ("fork",             "Horquilla"),
    ("pin",              "Clavada"),
    ("skewer",           "Ensartada"),
    ("discoveredAttack", "Ataque Descubierto"),
    ("doubleCheck",      "Jaque Doble"),
    ("hangingPiece",     "Pieza Colgada"),
    ("sacrifice",        "Sacrificio"),
    ("promotion",        "Promoción"),
    ("advancedPawn",     "Peón Avanzado"),
    ("endgame",          "Final"),
    ("rookEndgame",      "Final de Torres"),
    ("queenEndgame",     "Final de Dama"),
    ("pawnEndgame",      "Final de Peones"),
    ("kingsideAttack",   "Ataque al Flanco Rey"),
    ("queensideAttack",  "Ataque al Flanco Dama"),
    ("exposedKing",      "Rey Expuesto"),
    ("crushing",         "Ventaja Aplastante"),
    ("trappedPiece",     "Pieza Atrapada"),
    ("quietMove",        "Jugada Silenciosa"),
    ("middlegame",       "Juego Medio"),
    ("opening",          "Apertura"),
]

THEME_DESCRIPTIONS = {
    "mateIn1":          "Encontrá el jaque mate en un movimiento.",
    "mateIn2":          "Calculá la secuencia de jaque mate en dos movimientos.",
    "mateIn3":          "Encontrá el camino al jaque mate en tres movimientos.",
    "backRankMate":     "El rey enemigo está atrapado en la última fila por sus propias piezas.",
    "smotheredMate":    "Un caballo da jaque mate a un rey rodeado por sus propias piezas.",
    "fork":             "Atacá dos piezas enemigas al mismo tiempo con una sola pieza.",
    "pin":              "Una pieza no puede moverse porque expone a una más valiosa detrás.",
    "skewer":           "Atacá una pieza valiosa que al moverse expone otra menos valiosa.",
    "discoveredAttack": "Al mover una pieza, revelás un ataque de la pieza que estaba detrás.",
    "doubleCheck":      "Dos piezas dan jaque simultáneamente, forzando al rey a moverse.",
    "hangingPiece":     "Capturá una pieza enemiga que está desprotegida.",
    "sacrifice":        "Sacrificá material para obtener una ventaja posicional o de ataque.",
    "promotion":        "Avanzá un peón hasta la última fila y promovelo a una pieza superior.",
    "advancedPawn":     "Usá un peón avanzado como arma táctica o de promoción.",
    "endgame":          "Técnica de final con pocas piezas donde cada movimiento cuenta.",
    "rookEndgame":      "Final de torres donde la actividad y la posición del rey son clave.",
    "queenEndgame":     "Final de dama con ventaja de material o posicional.",
    "pawnEndgame":      "Final de peones donde la oposición y la promoción son decisivas.",
    "kingsideAttack":   "Lanzá un ataque decisivo sobre el flanco rey del enemigo.",
    "queensideAttack":  "Creá una amenaza irresistible en el flanco dama.",
    "exposedKing":      "El rey enemigo está expuesto en el centro o sin enrocar.",
    "crushing":         "Encontrá el golpe táctico que da una ventaja aplastante.",
    "trappedPiece":     "Atrapá una pieza enemiga que no tiene escapatoria.",
    "quietMove":        "La mejor jugada no es obvia — no da jaque ni captura.",
    "middlegame":       "Posición de juego medio con múltiples factores tácticos.",
    "opening":          "Error en la apertura que puede ser explotado inmediatamente.",
}

def classify(themes: list[str]) -> tuple[str, str, str]:
    """Returns (title, description, theme_key)."""
    for key, title in THEME_TITLES:
        if key in themes:
            desc = THEME_DESCRIPTIONS.get(key, "Encontrá la mejor jugada táctica.")
            return title, desc, key
    return "Táctica", "Encontrá la mejor jugada en esta posición.", "tactics"

def download_and_process(target: int) -> list[dict]:
    import chess
    import zstandard as zstd

    print(f"Descargando puzzles de Lichess (objetivo: {target})...")
    print(f"URL: {LICHESS_URL}")

    import subprocess, tempfile

    # Download via curl (avoids macOS SSL cert issues with urllib)
    tmp = tempfile.NamedTemporaryFile(suffix=".csv.zst", delete=False)
    tmp.close()
    print(f"  Descargando a {tmp.name} ...")
    subprocess.run(
        ["curl", "-L", "--silent", "--show-error", "-o", tmp.name, LICHESS_URL],
        check=True,
    )

    lessons = []
    skipped = 0
    processed = 0

    try:
        with open(tmp.name, "rb") as f:
            dctx = zstd.ZstdDecompressor()
            reader = dctx.stream_reader(f)
            text_reader = io.TextIOWrapper(reader, encoding="utf-8")

            # Skip header line
            text_reader.readline()

            for line in text_reader:
                if len(lessons) >= target:
                    break

                line = line.strip()
                if not line:
                    continue

                parts = line.split(",")
                if len(parts) < 8:
                    continue

                fen       = parts[1]
                moves_str = parts[2]
                themes_str = parts[7] if len(parts) > 7 else ""

                try:
                    rating = int(parts[3])
                except ValueError:
                    continue

                # Filter: good puzzle difficulty range
                if rating < 800 or rating > 2400:
                    skipped += 1
                    continue

                moves = moves_str.strip().split()
                if len(moves) < 2:
                    skipped += 1
                    continue

                themes = themes_str.strip().split()
                if not themes:
                    skipped += 1
                    continue

                # Apply opponent's setup move to get the actual puzzle position
                try:
                    board = chess.Board(fen)
                    setup_move = chess.Move.from_uci(moves[0])
                    if setup_move not in board.legal_moves:
                        skipped += 1
                        continue
                    board.push(setup_move)
                    puzzle_fen = board.fen()
                    answer_uci = moves[1]

                    answer_move = chess.Move.from_uci(answer_uci)
                    if answer_move not in board.legal_moves:
                        skipped += 1
                        continue
                except Exception:
                    skipped += 1
                    continue

                title, description, theme_key = classify(themes)

                lessons.append({
                    "title":       title,
                    "description": description,
                    "theme":       theme_key,
                    "fen":         puzzle_fen,
                    "answer_uci":  answer_uci,
                })

                processed += 1
                if processed % 1000 == 0:
                    print(f"  {processed}/{target} procesados (saltados: {skipped})...")
    finally:
        os.remove(tmp.name)

    print(f"Completado: {len(lessons)} lecciones, {skipped} saltadas.")
    return lessons

if __name__ == "__main__":
    target = int(sys.argv[1]) if len(sys.argv) > 1 else 10000

    lessons = download_and_process(target)

    os.makedirs(os.path.dirname(OUTPUT_PATH), exist_ok=True)
    with open(OUTPUT_PATH, "w", encoding="utf-8") as f:
        json.dump({"lessons": lessons}, f, ensure_ascii=False, separators=(",", ":"))

    size_mb = os.path.getsize(OUTPUT_PATH) / 1024 / 1024
    print(f"\nEscrito: {OUTPUT_PATH}")
    print(f"Lecciones: {len(lessons)}")
    print(f"Tamaño: {size_mb:.1f} MB")
