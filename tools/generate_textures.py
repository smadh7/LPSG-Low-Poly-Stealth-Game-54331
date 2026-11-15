#!/usr/bin/env python3
"""Generate the Coral Palm Resort PBR texture library.

Every material in the game is authored here as a small, tileable set of
physically based maps:

    <name>_basecolor.png   sRGB albedo
    <name>_normal.png      tangent-space normal map (OpenGL / +Y convention)
    <name>_orm.png         R = ambient occlusion, G = roughness, B = metallic

The maps are generated rather than downloaded so the repository stays
self-contained, license-clean and free of binary blobs of unknown provenance.
The output is deterministic (fixed seed), so regenerating changes nothing.

Usage:
    python3 tools/generate_textures.py [--out assets/textures] [--size 512]
"""

from __future__ import annotations

import argparse
import math
import os

import numpy as np
from PIL import Image

TILE = 2.0  # metres covered by one texture repeat


# ---------------------------------------------------------------------------
# Noise helpers
# ---------------------------------------------------------------------------
def _spectrum(size: int, rng: np.random.Generator, exponent: float) -> np.ndarray:
    """Band-limited noise via a 1/f^exponent spectrum in the frequency domain."""
    white = rng.standard_normal((size, size))
    freq = np.fft.fftfreq(size)
    fx, fy = np.meshgrid(freq, freq)
    radius = np.sqrt(fx**2 + fy**2)
    radius[0, 0] = 1.0
    field = np.fft.ifft2(white * (1.0 / radius**exponent)).real
    return field


def fbm(size: int, rng: np.random.Generator, exponent: float = 2.2, octaves: int = 1) -> np.ndarray:
    """Tileable fractal noise normalised to 0..1."""
    total = np.zeros((size, size))
    amp = 1.0
    for _ in range(octaves):
        total += _spectrum(size, rng, exponent) * amp
        amp *= 0.5
    total -= total.min()
    top = total.max()
    return total / top if top > 0 else total


def normalise(a: np.ndarray) -> np.ndarray:
    a = a - a.min()
    top = a.max()
    return a / top if top > 0 else a


def height_to_normal(height: np.ndarray, strength: float) -> np.ndarray:
    """Sobel-free central-difference normal map, tileable via np.roll."""
    dx = (np.roll(height, -1, axis=1) - np.roll(height, 1, axis=1)) * strength
    dy = (np.roll(height, -1, axis=0) - np.roll(height, 1, axis=0)) * strength
    nx, ny, nz = -dx, -dy, np.ones_like(height)
    length = np.sqrt(nx**2 + ny**2 + nz**2)
    out = np.stack([nx / length, ny / length, nz / length], axis=-1)
    return out * 0.5 + 0.5


def to_srgb(linear: np.ndarray) -> np.ndarray:
    linear = np.clip(linear, 0.0, 1.0)
    return np.where(linear <= 0.0031308, linear * 12.92, 1.055 * linear ** (1 / 2.4) - 0.055)


def save_rgb(path: str, arr: np.ndarray) -> None:
    Image.fromarray((np.clip(arr, 0.0, 1.0) * 255).astype(np.uint8), "RGB").save(path, optimize=True)


def save_gray(path: str, arr: np.ndarray) -> None:
    Image.fromarray((np.clip(arr, 0.0, 1.0) * 255).astype(np.uint8), "L").save(path, optimize=True)


# ---------------------------------------------------------------------------
# Small authoring language for surfaces
# ---------------------------------------------------------------------------
class Surface:
    """A base colour plus roughness / metallic / occlusion / height fields."""

    def __init__(self, size: int):
        self.size = size
        self.albedo = np.full((size, size, 3), 0.5)
        self.rough = np.full((size, size), 0.7)
        self.metal = np.zeros((size, size))
        self.height = np.zeros((size, size))
        self.ao = np.ones((size, size))


def grout_mask(size: int, rows: int, cols: int, width: float, offset_rows: bool = False) -> np.ndarray:
    """1.0 inside a tile, 0.0 in the mortar joints between tiles.

    `width` is the joint half-width as a fraction of one tile.
    """
    ys = np.linspace(0.0, rows, size, endpoint=False)
    xs = np.linspace(0.0, cols, size, endpoint=False)
    fy = (ys % 1.0).reshape(-1, 1)
    fx = (xs % 1.0).reshape(1, -1)
    if offset_rows:
        row = np.floor(ys).reshape(-1, 1)
        fx = (xs.reshape(1, -1) + 0.5 * (row % 2)) % 1.0
    # Distance to the nearest joint, measured from the tile centre outwards.
    edge_y = np.minimum(fy, 1.0 - fy)
    edge_x = np.minimum(fx, 1.0 - fx)
    distance = np.minimum(edge_y, edge_x)
    # Soft edge over one texel keeps mip-mapping from shimmering.
    feather = max(1.0 / size, 1e-6) * max(rows, cols)
    return np.clip((distance - width) / feather, 0.0, 1.0)


def tile_variance(size: int, rows: int, cols: int, rng: np.random.Generator) -> np.ndarray:
    """Tileable per-tile random field in 0..1 (stone-by-stone colour variation)."""
    coarse = rng.random((rows, cols))
    ys = (np.arange(size) * rows // size) % rows
    xs = (np.arange(size) * cols // size) % cols
    return coarse[ys][:, xs]


def mortar_grain(size: int, rng, scale: float, amount: float) -> np.ndarray:
    return fbm(size, rng, 2.6) * amount * scale


# ---------------------------------------------------------------------------
# Material definitions — each returns a Surface
# ---------------------------------------------------------------------------
def mat_grass(size, rng):
    """Lawn: dense clumps of blades at two scales plus dry patches."""
    s = Surface(size)
    clumps = fbm(size, rng, 2.6)
    # High-frequency, low-exponent noise reads as individual blades.
    blades = fbm(size, rng, 0.85, octaves=2)
    fine = fbm(size, rng, 0.55)
    tone = 0.30 * clumps + 0.42 * blades + 0.28 * fine
    dark = np.array([0.052, 0.115, 0.036])
    light = np.array([0.255, 0.400, 0.120])
    s.albedo = dark + (light - dark) * tone[..., None]
    # Sun-bleached patches.
    dry = np.clip((clumps - 0.62) * 2.4, 0.0, 1.0)[..., None]
    s.albedo = s.albedo * (1.0 - dry * 0.55) + np.array([0.360, 0.330, 0.150]) * dry * 0.55
    s.height = 0.75 * blades + 0.25 * fine
    s.rough = 0.88 + 0.10 * fine
    s.ao = (0.45 + 0.55 * blades) * (0.75 + 0.25 * clumps)
    return s


def mat_sand(size, rng):
    """Beach sand: fine grain plus soft wind ripples, no directional banding."""
    s = Surface(size)
    grain = fbm(size, rng, 0.45)
    medium = fbm(size, rng, 1.5, octaves=2)
    patches = fbm(size, rng, 2.8)
    # Faint, irregular dunes rather than regular wind ripples.
    phase = (np.arange(size).reshape(-1, 1) * 0.045) + patches * 22.0 + medium * 9.0
    ripples = 0.5 + 0.5 * np.sin(phase) * (0.35 + 0.65 * medium)
    speckle = fbm(size, rng, 0.30)
    tone = 0.34 * grain + 0.14 * ripples + 0.28 * patches + 0.24 * speckle
    s.albedo = np.stack([
        0.680 + 0.20 * tone,
        0.600 + 0.20 * tone,
        0.430 + 0.19 * tone,
    ], axis=-1)
    s.height = 0.20 * ripples + 0.80 * grain
    s.rough = 0.94 - 0.04 * grain
    s.ao = 0.93 + 0.07 * medium
    return s


def mat_marble(size, rng):
    """Calacatta-style marble: broad soft bands crossed by a few sharp veins."""
    s = Surface(size)
    # Low-frequency flow field: this is what makes marble read as stone rather
    # than noise, so it uses a smooth spectrum (few octaves, steep falloff).
    flow = fbm(size, rng, 3.0, octaves=3)
    warp = fbm(size, rng, 3.6, octaves=2)
    detail = fbm(size, rng, 2.0, octaves=2)
    field = flow + 0.85 * warp + 0.25 * detail

    # Broad grey banding.
    bands = 0.5 + 0.5 * np.sin(field * 4.2 + detail * 2.0)
    base = np.array([0.925, 0.912, 0.888]) - 0.055 * bands[..., None]

    # Primary veins: thin, dark, meandering.
    vein_field = field * 3.4 + detail * 3.0
    vein = np.clip(1.0 - np.abs(np.sin(vein_field)) * 34.0, 0.0, 1.0)
    vein = vein ** 1.4
    # Secondary hairline veins, sparser.
    hair = np.clip(1.0 - np.abs(np.sin(vein_field * 2.3 + 1.7)) * 60.0, 0.0, 1.0)

    s.albedo = base - vein[..., None] * np.array([0.34, 0.335, 0.345])
    s.albedo -= hair[..., None] * np.array([0.16, 0.16, 0.17])
    s.albedo = np.clip(s.albedo, 0.0, 1.0)
    s.height = 0.85 - 0.7 * vein - 0.2 * hair
    s.rough = 0.085 + 0.20 * vein + 0.06 * bands
    s.ao = 1.0 - 0.12 * vein
    return s


def mat_limestone(size, rng):
    """Large pale honed stone slabs — the main plaza / lobby floor."""
    s = Surface(size)
    tiles = grout_mask(size, 2, 3, 0.018)
    grain = fbm(size, rng, 1.6, octaves=2)
    blotch = fbm(size, rng, 2.6)
    base = np.array([0.815, 0.790, 0.735]) * (0.93 + 0.12 * blotch)[..., None]
    grout = np.array([0.50, 0.48, 0.45])
    s.albedo = np.where(tiles[..., None] > 0.5, base, grout)
    s.albedo *= (0.96 + 0.08 * grain)[..., None]
    s.height = tiles * 0.8 + grain * 0.2
    s.rough = np.where(tiles > 0.5, 0.42 + 0.10 * grain, 0.85)
    s.ao = np.where(tiles > 0.5, 1.0, 0.72) * (0.95 + 0.05 * blotch)
    return s


def mat_travertine(size, rng):
    """Warm travertine wall cladding: horizontal courses with pitting."""
    s = Surface(size)
    courses = grout_mask(size, 4, 1, 0.010)
    pits = fbm(size, rng, 1.3, octaves=3)
    flow = fbm(size, rng, 2.8)
    base = np.array([0.735, 0.660, 0.520]) * (0.90 + 0.16 * flow)[..., None]
    holes = np.clip((pits - 0.80) * 6.0, 0.0, 1.0)
    s.albedo = np.where(courses[..., None] > 0.5, base, np.array([0.45, 0.40, 0.32]))
    s.albedo *= (1.0 - 0.45 * holes)[..., None]
    s.height = courses * 0.7 + 0.3 * (1.0 - holes)
    s.rough = np.where(courses > 0.5, 0.72 + 0.10 * flow, 0.90)
    s.ao = np.where(courses > 0.5, 1.0, 0.75) * (1.0 - 0.5 * holes)
    return s


def mat_plaster(size, rng):
    """Painted stucco: soft, slightly mottled, eggshell finish."""
    s = Surface(size)
    mottle = fbm(size, rng, 2.5)
    trowel = fbm(size, rng, 1.5, octaves=2)
    s.albedo = np.array([0.885, 0.865, 0.820]) * (0.95 + 0.08 * mottle)[..., None]
    s.height = 0.45 * trowel + 0.55 * mottle
    s.rough = 0.66 + 0.10 * mottle
    s.ao = 0.92 + 0.08 * trowel
    return s


def mat_wood(size, rng):
    """Teak decking / furniture: long grain with plank grooves."""
    s = Surface(size)
    planks = grout_mask(size, 6, 1, 0.012)
    warp = fbm(size, rng, 2.2)
    rings = np.sin((np.linspace(0, 1, size).reshape(-1, 1) * 34.0 + warp * 5.0)) * 0.5 + 0.5
    fibre = fbm(size, rng, 0.7)
    tone = 0.45 * rings + 0.30 * fibre + 0.25 * warp
    dark = np.array([0.135, 0.072, 0.034])
    light = np.array([0.330, 0.190, 0.100])
    s.albedo = dark + (light - dark) * tone[..., None]
    s.albedo = np.where(planks[..., None] > 0.5, s.albedo, s.albedo * 0.35)
    s.height = planks * 0.75 + 0.25 * fibre
    s.rough = np.where(planks > 0.5, 0.48 + 0.16 * fibre, 0.85)
    s.ao = np.where(planks > 0.5, 1.0, 0.55)
    return s


def mat_terracotta(size, rng):
    """Hand-made terracotta floor tiles: warm, uneven, visibly individual."""
    s = Surface(size)
    rows, cols = 5, 5
    tiles = grout_mask(size, rows, cols, 0.028)
    tone = tile_variance(size, rows, cols, rng)
    grain = fbm(size, rng, 0.8, octaves=2)
    stain = fbm(size, rng, 2.4)
    base = np.array([0.560, 0.215, 0.105])
    s.albedo = base * (0.72 + 0.60 * tone)[..., None]
    s.albedo *= (0.86 + 0.24 * stain)[..., None]
    s.albedo = np.where(tiles[..., None] > 0.5, s.albedo, np.array([0.245, 0.220, 0.195]))
    s.height = tiles * 0.85 + 0.15 * grain
    s.rough = np.where(tiles > 0.5, 0.66 + 0.16 * grain, 0.92)
    s.ao = np.where(tiles > 0.5, 1.0 - 0.10 * (1.0 - stain), 0.55)
    return s


def mat_concrete(size, rng):
    s = Surface(size)
    blotch = fbm(size, rng, 2.8)
    pits = fbm(size, rng, 0.8)
    joints = grout_mask(size, 1, 2, 0.006)
    s.albedo = np.array([0.560, 0.545, 0.515]) * (0.88 + 0.20 * blotch)[..., None]
    s.albedo *= (1.0 - 0.20 * np.clip((pits - 0.8) * 5.0, 0.0, 1.0))[..., None]
    s.albedo = np.where(joints[..., None] > 0.5, s.albedo, s.albedo * 0.65)
    s.height = 0.3 * pits + 0.7 * joints
    s.rough = 0.80 + 0.12 * blotch
    s.ao = (0.90 + 0.10 * blotch) * np.where(joints > 0.5, 1.0, 0.7)
    return s


def mat_brick(size, rng):
    s = Surface(size)
    courses = grout_mask(size, 8, 4, 0.020, offset_rows=True)
    rough_grain = fbm(size, rng, 1.2, octaves=2)
    tint = fbm(size, rng, 2.6)
    base = np.array([0.430, 0.180, 0.115]) * (0.70 + 0.55 * tint)[..., None]
    s.albedo = np.where(courses[..., None] > 0.5, base, np.array([0.66, 0.63, 0.585]))
    s.height = courses * 0.9 + 0.1 * rough_grain
    s.rough = np.where(courses > 0.5, 0.82 + 0.08 * rough_grain, 0.92)
    s.ao = np.where(courses > 0.5, 1.0, 0.62)
    return s


def mat_slate(size, rng):
    s = Surface(size)
    tiles = grout_mask(size, 6, 3, 0.016)
    grain = fbm(size, rng, 1.0, octaves=2)
    tone = fbm(size, rng, 2.5)
    base = np.array([0.135, 0.145, 0.165]) * (0.75 + 0.45 * tone)[..., None]
    s.albedo = np.where(tiles[..., None] > 0.5, base, np.array([0.06, 0.065, 0.075]))
    s.height = tiles * 0.8 + 0.2 * grain
    s.rough = np.where(tiles > 0.5, 0.44 + 0.18 * grain, 0.80)
    s.ao = np.where(tiles > 0.5, 1.0, 0.55)
    return s


def mat_metal(size, rng):
    s = Surface(size)
    brushed = fbm(size, rng, 1.0)
    streaks = np.repeat(fbm(size, rng, 1.6).mean(axis=0)[None, :], size, axis=0)
    tone = 0.5 * brushed + 0.5 * streaks
    s.albedo = np.array([0.615, 0.625, 0.640]) * (0.86 + 0.22 * tone)[..., None]
    s.metal = np.full((size, size), 1.0)
    s.rough = 0.28 + 0.22 * tone
    s.height = tone * 0.3
    return s


def mat_stone_wall(size, rng):
    """Rough stacked stone: individual stones with real colour variation."""
    s = Surface(size)
    rows, cols = 9, 6
    courses = grout_mask(size, rows, cols, 0.026, offset_rows=True)
    tone = tile_variance(size, rows, cols, rng)
    grain = fbm(size, rng, 0.8, octaves=2)
    relief = fbm(size, rng, 1.6)
    base = np.array([0.520, 0.485, 0.420])
    # Warm-to-cool drift per stone keeps the wall from looking like wallpaper.
    variation = (tone[..., None] - 0.5) * np.array([0.34, 0.30, 0.22])
    stone = base + variation + (relief[..., None] - 0.5) * 0.16
    stone *= (0.82 + 0.36 * grain)[..., None]
    mortar = np.array([0.300, 0.285, 0.250])
    s.albedo = np.where(courses[..., None] > 0.5, stone, mortar)
    s.height = courses * 0.85 + 0.15 * relief
    s.rough = np.where(courses > 0.5, 0.80 + 0.12 * grain, 0.93)
    s.ao = np.where(courses > 0.5, 1.0 - 0.20 * (1.0 - relief), 0.42)
    return s


def mat_roof(size, rng):
    """Terracotta barrel roof tiles: horizontal courses of half-round tiles."""
    s = Surface(size)
    rows, cols = 7, 10
    course = grout_mask(size, rows, 1, 0.020)
    ys = np.linspace(0.0, rows, size, endpoint=False)
    xs = np.linspace(0.0, 1.0, size, endpoint=False)
    # Each tile is a half cylinder: shade across the tile's width.
    barrel = np.sin(xs * math.pi)[None, :] ** 0.6
    tile_tone = tile_variance(size, rows, cols, rng)
    grain = fbm(size, rng, 1.2, octaves=2)
    soot = fbm(size, rng, 2.2)
    base = np.array([0.470, 0.185, 0.115])
    s.albedo = base * (0.60 + 0.80 * tile_tone)[..., None]
    s.albedo *= (0.70 + 0.45 * barrel)[..., None]
    s.albedo *= (0.88 + 0.32 * soot)[..., None]
    s.albedo = np.where(course[..., None] > 0.5, s.albedo, s.albedo * 0.40)
    s.height = 0.75 * barrel * course + 0.25 * grain
    s.rough = 0.68 + 0.16 * grain
    s.ao = (0.55 + 0.45 * barrel) * np.where(course > 0.5, 1.0, 0.5)
    return s


def mat_hedge(size, rng):
    """Clipped hedge: dense small leaves with dark gaps between them."""
    s = Surface(size)
    leaves = fbm(size, rng, 0.42, octaves=3)
    clump = fbm(size, rng, 1.7)
    gaps = np.clip((fbm(size, rng, 0.45) - 0.55) * 2.6, 0.0, 1.0)
    tone = 0.55 * leaves + 0.45 * clump
    dark = np.array([0.018, 0.055, 0.018])
    light = np.array([0.175, 0.345, 0.105])
    s.albedo = dark + (light - dark) * tone[..., None]
    s.albedo *= (1.0 - 0.55 * gaps)[..., None]
    s.height = 0.8 * leaves + 0.2 * clump
    s.rough = 0.72 + 0.14 * leaves
    s.ao = (0.30 + 0.70 * leaves) * (0.60 + 0.40 * clump)
    return s


def mat_palm_bark(size, rng):
    s = Surface(size)
    rings = 0.5 + 0.5 * np.sin(np.linspace(0, 2 * math.pi * 14, size).reshape(-1, 1))
    grain = fbm(size, rng, 1.2, octaves=2)
    tone = 0.55 * rings + 0.45 * grain
    dark = np.array([0.115, 0.080, 0.052])
    light = np.array([0.300, 0.222, 0.150])
    s.albedo = dark + (light - dark) * tone[..., None]
    s.height = 0.75 * rings + 0.25 * grain
    s.rough = 0.82 + 0.10 * grain
    return s


def mat_frond(size, rng):
    """Palm frond: leaflets sweeping out at a shallow angle from a midrib."""
    s = Surface(size)
    ys = np.linspace(-1.0, 1.0, size).reshape(-1, 1)   # across the frond
    xs = np.linspace(0.0, 1.0, size).reshape(1, -1)    # along the frond
    jitter = fbm(size, rng, 2.0)
    # Leaflets leave the rachis at an angle, so the pattern is a shallow V.
    v = xs * 26.0 + np.abs(ys) * 9.0 + jitter * 2.5
    leaflets = 0.5 + 0.5 * np.sin(v * math.tau)
    leaflet_mask = np.clip((leaflets - 0.28) * 2.4, 0.0, 1.0)
    rib = np.clip(1.0 - np.abs(ys) * 11.0, 0.0, 1.0)
    grain = fbm(size, rng, 0.9, octaves=2)
    tone = 0.55 * leaflet_mask + 0.45 * grain
    dark = np.array([0.040, 0.090, 0.028])
    light = np.array([0.190, 0.330, 0.100])
    s.albedo = dark + (light - dark) * tone[..., None]
    # Pale woody rachis down the middle.
    s.albedo = s.albedo * (1.0 - rib[..., None] * 0.85) + np.array([0.330, 0.300, 0.170]) * rib[..., None] * 0.85
    s.height = 0.8 * leaflet_mask + 0.2 * grain
    s.rough = 0.58 + 0.14 * grain
    s.ao = (0.35 + 0.65 * leaflet_mask) * (0.8 + 0.2 * grain)
    return s


def mat_skin(size, rng):
    """Skin: subsurface-ish blotching, pores and a little redness variation."""
    s = Surface(size)
    pores = fbm(size, rng, 0.55)
    blotch = fbm(size, rng, 2.6)
    redness = fbm(size, rng, 3.4)
    base = np.array([0.640, 0.462, 0.372])
    s.albedo = base * (0.93 + 0.10 * blotch)[..., None]
    # Slight red/olive drift, the thing that stops skin reading as plastic.
    s.albedo += (redness[..., None] - 0.5) * np.array([0.075, 0.012, -0.020])
    s.albedo *= (1.0 - 0.06 * pores)[..., None]
    s.albedo = np.clip(s.albedo, 0.0, 1.0)
    s.rough = 0.44 + 0.14 * pores + 0.05 * blotch
    s.height = pores * 0.30
    s.ao = 0.94 + 0.06 * blotch
    return s


def mat_fabric(size, rng):
    """Woven cloth: regular warp/weft threads with a soft slub pattern."""
    s = Surface(size)
    warp = 0.5 + 0.5 * np.sin(np.linspace(0, 2 * math.pi * 64, size)).reshape(1, -1)
    weft = 0.5 + 0.5 * np.sin(np.linspace(0, 2 * math.pi * 64, size)).reshape(-1, 1)
    weave = np.maximum(warp * 0.55, weft * 0.55)
    slub = fbm(size, rng, 1.8)
    tone = 0.5 * weave + 0.5 * slub
    s.albedo = np.array([0.80, 0.80, 0.79]) * (0.80 + 0.35 * tone)[..., None]
    s.height = 0.6 * weave + 0.4 * slub
    s.rough = 0.86 + 0.08 * slub
    s.ao = 0.85 + 0.15 * weave
    return s


def mat_water_normal(size, rng):
    s = Surface(size)
    a = fbm(size, rng, 1.9)
    b = fbm(size, rng, 1.3)
    s.height = 0.55 * a + 0.45 * b
    s.albedo = np.full((size, size, 3), 0.5)
    s.rough = 0.05 + 0.05 * a
    return s


def flat(size, rgb, rough=0.6, metal=0.0, noise=0.0, rng=None):
    s = Surface(size)
    base = np.array(rgb)
    if noise > 0.0 and rng is not None:
        tint = fbm(size, rng, 2.4)
        s.albedo = base * (1.0 - noise + 2.0 * noise * tint)[..., None]
        s.height = tint * 0.2
    else:
        s.albedo = np.broadcast_to(base, (size, size, 3)).copy()
    s.rough = np.full((size, size), rough)
    s.metal = np.full((size, size), metal)
    return s


# name -> (builder, surface normal strength, texture repeat in metres)
MATERIALS = {
    "grass": (mat_grass, 1.6, 2.5),
    "sand": (mat_sand, 1.2, 3.0),
    "marble": (mat_marble, 1.2, 2.5),
    "limestone": (mat_limestone, 2.0, 3.0),
    "travertine": (mat_travertine, 2.2, 2.5),
    "plaster": (mat_plaster, 0.9, 3.0),
    "wood": (mat_wood, 2.0, 2.0),
    "terracotta": (mat_terracotta, 2.4, 2.0),
    "concrete": (mat_concrete, 1.4, 3.0),
    "brick": (mat_brick, 2.6, 1.5),
    "slate": (mat_slate, 2.0, 2.0),
    "metal": (mat_metal, 0.8, 2.0),
    "stone_wall": (mat_stone_wall, 3.0, 2.5),
    "roof": (mat_roof, 2.6, 4.0),
    "hedge": (mat_hedge, 2.4, 1.5),
    "palm_bark": (mat_palm_bark, 1.6, 1.0),
    "frond": (mat_frond, 1.0, 1.0),
    "skin": (mat_skin, 0.6, 0.5),
    "fabric": (mat_fabric, 1.0, 0.5),
    "water_normal": (mat_water_normal, 2.6, 8.0),
    "white_paint": (lambda z, r: flat(z, (0.90, 0.90, 0.88), 0.35, noise=0.03, rng=r), 0.4, 2.0),
    "dark_trim": (lambda z, r: flat(z, (0.085, 0.085, 0.095), 0.45, noise=0.05, rng=r), 0.4, 1.0),
    "brass": (lambda z, r: flat(z, (0.72, 0.55, 0.28), 0.28, metal=1.0, noise=0.08, rng=r), 0.4, 1.0),
    "rubber": (lambda z, r: flat(z, (0.045, 0.045, 0.048), 0.75, noise=0.06, rng=r), 0.4, 1.0),
    "asphalt": (lambda z, r: flat(z, (0.075, 0.075, 0.078), 0.85, noise=0.30, rng=r), 1.2, 4.0),
}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default="assets/textures")
    parser.add_argument("--size", type=int, default=512)
    args = parser.parse_args()
    os.makedirs(args.out, exist_ok=True)
    manifest = []

    for index, (name, (builder, strength, tiling)) in enumerate(sorted(MATERIALS.items())):
        rng = np.random.default_rng(1337 + index)
        surface = builder(args.size, rng)
        save_rgb(os.path.join(args.out, f"{name}_basecolor.png"), to_srgb(surface.albedo))
        # File naming: the shader path is `<name>_normal.png`; the loader builds
        # `<name>_normal.png` from the material name, so keep the pattern.
        save_rgb(os.path.join(args.out, f"{name}_normal.png"), height_to_normal(surface.height, strength * 6.0))
        orm = np.stack([np.clip(surface.ao, 0, 1), np.clip(surface.rough, 0, 1), np.clip(surface.metal, 0, 1)], axis=-1)
        save_rgb(os.path.join(args.out, f"{name}_orm.png"), orm)
        manifest.append((name, tiling))
        print(f"  {name:<14} tiling {tiling:.1f} m")

    # Rust-side table so the renderer and the generator cannot drift apart.
    with open(os.path.join(args.out, "TILING.txt"), "w") as fh:
        fh.write("# material, metres per texture repeat\n")
        for name, tiling in manifest:
            fh.write(f"{name} {tiling}\n")
    print(f"\ngenerated {len(manifest)} materials into {args.out}")


if __name__ == "__main__":
    main()
