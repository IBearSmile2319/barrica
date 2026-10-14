"""Música y efectos originales del vídeo (sin pistas de terceros): 110 BPM, La menor, Am–F–C–G.
Los cortes de escena caen en tiempos (ver brag-plan.md). Escribe soundtrack.wav (48 kHz, estéreo)."""
import numpy as np
from scipy.io import wavfile
from scipy.signal import butter, sosfilt

SR = 48000
BPM = 110
BEAT = 60 / BPM
DUR = 38 * BEAT  # 20,73 s
N = int(DUR * SR) + SR // 2
rng = np.random.default_rng(7)

def midi(n):
    return 440.0 * 2 ** ((n - 69) / 12)

def env(n, a=0.005, d=0.3, s=0.0, r=0.05):
    t = np.arange(n) / SR
    e = np.minimum(1, t / a) * (s + (1 - s) * np.exp(-t / d))
    tail = int(r * SR)
    if tail and tail < n:
        e[-tail:] *= np.linspace(1, 0, tail)
    return e

def add(buf, start, sig, pan=0.0):
    i = int(start * SR)
    j = min(len(buf), i + len(sig))
    if i >= len(buf):
        return
    l, r = np.cos((pan + 1) * np.pi / 4), np.sin((pan + 1) * np.pi / 4)
    buf[i:j, 0] += sig[: j - i] * l
    buf[i:j, 1] += sig[: j - i] * r

def lp(x, f):
    return sosfilt(butter(2, f, "low", fs=SR, output="sos"), x)

def hp(x, f):
    return sosfilt(butter(2, f, "high", fs=SR, output="sos"), x)

def saw(f, n):
    t = np.arange(n) / SR
    return 2 * ((t * f) % 1) - 1

music = np.zeros((N, 2))
sfx = np.zeros((N, 2))

# Acordes (raíz MIDI, notas): Am, F, C, G; un acorde por compás (4 tiempos).
CHORDS = [(57, [57, 60, 64]), (53, [53, 57, 60]), (48, [55, 60, 64]), (55, [55, 59, 62])]
INTRO = 5  # la escena 1 (terminal) va con poca música; todo entra en el tiempo 5

for beat in range(38):
    t0 = beat * BEAT
    bar = beat // 4
    root, notes = CHORDS[bar % 4]
    full = beat >= INTRO
    # Bombo: suave, en cada tiempo cuando entra todo.
    if full and beat < 36:
        n = int(0.35 * SR)
        tt = np.arange(n) / SR
        k = np.sin(2 * np.pi * (48 + 90 * np.exp(-tt * 28)) * tt) * env(n, 0.002, 0.12)
        add(music, t0, 0.55 * k)
    # Hi-hat a contratiempo.
    if beat >= 2 and beat < 36:
        n = int(0.06 * SR)
        h = hp(rng.standard_normal(n), 7000) * env(n, 0.001, 0.02)
        add(music, t0 + BEAT / 2, (0.10 if full else 0.05) * h, pan=0.25)
    # Bajo: raíz en corcheas.
    if full and beat < 36:
        for half in (0, 1):
            n = int(BEAT / 2 * SR)
            b = lp(saw(midi(root - 12), n), 420) * env(n, 0.005, 0.18, 0.4, 0.03)
            add(music, t0 + half * BEAT / 2, 0.22 * b)
    # Pad: un acorde por compás, también en la intro (más bajo).
    if beat % 4 == 0 and beat < 36:
        n = int(4 * BEAT * SR)
        p = sum(lp(saw(midi(m) * d, n), 1800) for m in notes for d in (0.997, 1.003))
        p *= env(n, 0.35, 10, 0.9, 0.4)
        add(music, t0, (0.035 if full else 0.02) * p, pan=-0.15)
    # Arpegio en semicorcheas (escenas 3–5).
    if 10 <= beat < 32:
        arp = notes + [notes[1] + 12]
        for k in range(4):
            n = int(BEAT / 4 * SR)
            f = midi(arp[(beat * 4 + k) % 4] + 12)
            a = np.sin(2 * np.pi * f * np.arange(n) / SR) * env(n, 0.003, 0.08)
            add(music, t0 + k * BEAT / 4, 0.05 * a, pan=0.3 if k % 2 else -0.3)

# Final: acorde de La menor que se apaga.
n = int(3.4 * SR)
fin = sum(np.sin(2 * np.pi * midi(m) * np.arange(n) / SR) for m in (45, 57, 60, 64, 69)) * env(n, 0.01, 1.2)
add(music, 36 * BEAT, 0.07 * fin)

# --- Efectos, en la misma tonalidad y por debajo de la música ---
def tick(t, gain=0.05):
    n = int(0.03 * SR)
    add(sfx, t, gain * hp(rng.standard_normal(n), 3000) * env(n, 0.0005, 0.006), pan=rng.uniform(-0.3, 0.3))

def ding(t, note, gain=0.09):
    n = int(1.2 * SR)
    tt = np.arange(n) / SR
    s = (np.sin(2 * np.pi * midi(note) * tt) + 0.3 * np.sin(2 * np.pi * midi(note + 12) * tt)) * env(n, 0.002, 0.35)
    add(sfx, t, gain * s)

def whoosh(t, dur=0.6, gain=0.12):
    n = int(dur * SR)
    noise = rng.standard_normal(n)
    out = np.zeros(n)
    for i, f in enumerate(np.linspace(400, 5000, 6)):
        seg = slice(i * n // 6, (i + 1) * n // 6)
        out[seg] = lp(noise, f)[seg]
    add(sfx, t - dur * 0.7, gain * out * np.sin(np.linspace(0, np.pi, n)))

def boom(t):
    n = int(1.4 * SR)
    tt = np.arange(n) / SR
    add(sfx, t, 0.5 * np.sin(2 * np.pi * (38 + 40 * np.exp(-tt * 10)) * tt) * env(n, 0.003, 0.45))

# Escena 1: tecleo del comando y un tick por línea de log.
for i in range(18):
    tick(0.25 + i * 0.045)
for t in np.arange(1.15, 2.2, 0.15):
    tick(t, 0.035)
ding(2.25, 69, 0.07)
# Cortes y momentos clave (tiempos de brag-plan.md).
boom(5 * BEAT)
for t in (5 * BEAT, 10 * BEAT, 18 * BEAT, 26 * BEAT, 32 * BEAT):
    whoosh(t)
for i, note in enumerate((64, 69, 72)):  # tarjetas de capas
    ding(10 * BEAT + 1.1 + i * 0.45, note)
for i, note in enumerate((69, 76)):  # parches
    ding(26 * BEAT + 0.45 + i * 0.4, note)

mix = 0.9 * music + 0.6 * sfx
mix = lp(mix.T, 16000).T
peak = np.max(np.abs(mix))
mix = np.tanh(1.2 * mix / peak) / np.tanh(1.2) * 0.89  # limitador suave
fade = int(0.25 * SR)
mix[-fade:] *= np.linspace(1, 0, fade)[:, None]
wavfile.write("soundtrack.wav", SR, (mix[: int(DUR * SR)] * 32767).astype(np.int16))
print(f"soundtrack.wav: {DUR:.2f} s")
