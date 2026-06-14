"""
Wind Tunnel — Publication Panel Generator
==========================================
Five panels, four charts each, white background, minimal text.
At least one 3D chart per panel. All charts are numerical.

Panel 1  — S-functional and action-cell          (E01–E04)
Panel 2  — Local Invisibility and No Template    (E05–E08)
Panel 3  — Holonomy                              (E09–E12)
Panel 4  — Kuramoto and coordination regimes     (E15–E19)
Panel 5  — Protocol: tension, decoherence, scores(E21–E25)
"""

import math, cmath, random, json
from pathlib import Path
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
from matplotlib import cm
from mpl_toolkits.mplot3d import Axes3D  # noqa: F401

OUT_DIR = Path(__file__).parent.parent / "publications" / "windtunnel-code-testing" / "figures"
OUT_DIR.mkdir(exist_ok=True)

# ── colour palette (colourblind-safe) ────────────────────────────────────────
C0 = "#2166AC"   # steel blue
C1 = "#D6604D"   # crimson
C2 = "#4DAC26"   # sea green
C3 = "#B2ABD2"   # lavender
C4 = "#F4A582"   # salmon
REGIME_COLS = {
    "turbulent":           "#D6604D",
    "aperture_dominated":  "#F4A582",
    "hierarchical_cascade":"#FFFFBF",
    "coherent":            "#A6D96A",
    "phase_locked":        "#2166AC",
}
SIGMA = 100.0

# ── shared style ─────────────────────────────────────────────────────────────
plt.rcParams.update({
    "figure.facecolor":  "white",
    "axes.facecolor":    "white",
    "axes.edgecolor":    "#444444",
    "axes.linewidth":    0.8,
    "axes.spines.top":   False,
    "axes.spines.right": False,
    "xtick.direction":   "out",
    "ytick.direction":   "out",
    "xtick.major.size":  3,
    "ytick.major.size":  3,
    "font.size":         8,
    "axes.titlesize":    8,
    "axes.titleweight":  "bold",
    "axes.labelsize":    7,
    "legend.fontsize":   6,
    "legend.frameon":    False,
    "lines.linewidth":   1.2,
})

def _save(fig, name):
    path = OUT_DIR / name
    fig.savefig(path, dpi=200, bbox_inches="tight", facecolor="white")
    plt.close(fig)
    print(f"  saved {path.name}")


# ─────────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────────

def _s_value(x, centre, radius, beta):
    dist = math.sqrt(sum((xi - ci)**2 for xi, ci in zip(x, centre)))
    return max(0.0, dist - radius) + beta

def _regime(R):
    if R < 0.3:   return "turbulent"
    if R < 0.5:   return "aperture_dominated"
    if R < 0.8:   return "hierarchical_cascade"
    if R < 0.95:  return "coherent"
    return "phase_locked"

def _regime_color(R):
    return REGIME_COLS[_regime(R)]

def _kuramoto_simulate(N, K, omegas, T=40.0, dt=0.02, seed=0):
    rng = random.Random(seed)
    thetas = [rng.uniform(0, 2*math.pi) for _ in range(N)]
    steps  = int(T / dt)
    R_series = []
    for step in range(steps):
        d = [omegas[i] + (K/N)*sum(math.sin(thetas[j]-thetas[i]) for j in range(N))
             for i in range(N)]
        thetas = [thetas[i] + dt*d[i] for i in range(N)]
        if step % 20 == 0:
            R_series.append(abs(sum(cmath.exp(1j*t) for t in thetas))/N)
    return thetas, R_series


# =============================================================================
# PANEL 1 — S-functional and action-cell
# A: S(x) vs distance from cell boundary (1D slice)
# B: 2D heat-map of S over the plane
# C: S histogram: inside vs outside cell
# D: 3D surface of S over (x,y)
# =============================================================================

def panel_1():
    fig, axes = plt.subplots(1, 4, figsize=(14, 3.2),
                             gridspec_kw={"width_ratios": [1, 1, 1, 1.4]})
    fig.subplots_adjust(wspace=0.42, left=0.06, right=0.97, top=0.88, bottom=0.18)

    centre = [0.0, 0.0]
    radius = 1.0
    beta   = 2.5
    rng    = random.Random(42)

    # ── A: S vs radial distance ───────────────────────────────────────────
    ax = axes[0]
    r_vals = np.linspace(0, 4, 300)
    S_vals = [max(0.0, r - radius) + beta for r in r_vals]
    ax.axvspan(0, radius, color=C2, alpha=0.15, lw=0)
    ax.plot(r_vals, S_vals, color=C0, lw=1.4)
    ax.axhline(beta, color=C1, lw=0.9, ls="--")
    ax.set_xlabel("radial distance  $r$")
    ax.set_ylabel("$\\mathcal{S}(x)$")
    ax.set_title("A   S vs distance")
    ax.set_ylim(0, 7)

    # ── B: 2D heat-map ───────────────────────────────────────────────────
    ax = axes[1]
    gx = np.linspace(-3, 3, 120)
    gy = np.linspace(-3, 3, 120)
    GX, GY = np.meshgrid(gx, gy)
    GS = np.vectorize(lambda x, y: _s_value([x, y], centre, radius, beta))(GX, GY)
    im = ax.contourf(GX, GY, GS, levels=20, cmap="viridis")
    theta_c = np.linspace(0, 2*math.pi, 200)
    ax.plot(np.cos(theta_c), np.sin(theta_c), color="white", lw=1.0, ls="--")
    fig.colorbar(im, ax=ax, pad=0.02, shrink=0.85)
    ax.set_aspect("equal")
    ax.set_xlabel("$x_1$");  ax.set_ylabel("$x_2$")
    ax.set_title("B   $\\mathcal{S}$ heat-map")

    # ── C: histogram inside vs outside ───────────────────────────────────
    ax = axes[2]
    S_in, S_out = [], []
    for _ in range(600):
        r_s   = rng.uniform(0, radius * 0.97)
        angle = rng.uniform(0, 2*math.pi)
        S_in.append(_s_value([r_s*math.cos(angle), r_s*math.sin(angle)],
                              centre, radius, beta))
    for _ in range(600):
        r_s   = rng.uniform(radius * 1.05, 4.0)
        angle = rng.uniform(0, 2*math.pi)
        S_out.append(_s_value([r_s*math.cos(angle), r_s*math.sin(angle)],
                               centre, radius, beta))
    bins = np.linspace(2.0, 7.0, 30)
    ax.hist(S_in,  bins=bins, color=C2, alpha=0.75, label="inside $C^*$")
    ax.hist(S_out, bins=bins, color=C0, alpha=0.55, label="outside $C^*$")
    ax.axvline(beta, color=C1, lw=0.9, ls="--")
    ax.set_xlabel("$\\mathcal{S}$");  ax.set_ylabel("count")
    ax.set_title("C   distribution of $\\mathcal{S}$")
    ax.legend()

    # ── D: 3D surface ────────────────────────────────────────────────────
    ax3d = fig.add_subplot(1, 4, 4, projection="3d",
                           facecolor="white")
    axes[3].set_visible(False)
    gx3 = np.linspace(-3, 3, 50)
    gy3 = np.linspace(-3, 3, 50)
    GX3, GY3 = np.meshgrid(gx3, gy3)
    GS3 = np.vectorize(lambda x, y: _s_value([x, y], centre, radius, beta))(GX3, GY3)
    ax3d.plot_surface(GX3, GY3, GS3, cmap="viridis",
                      rcount=50, ccount=50, alpha=0.92, linewidth=0)
    ax3d.set_xlabel("$x_1$", labelpad=2);  ax3d.set_ylabel("$x_2$", labelpad=2)
    ax3d.set_zlabel("$\\mathcal{S}$", labelpad=2)
    ax3d.set_title("D   3D surface of $\\mathcal{S}$", pad=4)
    ax3d.view_init(elev=28, azim=-55)
    ax3d.set_facecolor("white")

    _save(fig, "panel_1.png")


# =============================================================================
# PANEL 2 — Local Invisibility and No Template
# A: 3-cycle residual accumulation vs traversal count
# B: per-unit S snapshot (unit tests see nothing)
# C: observer capacity vs system complexity (pigeonhole)
# D: 3D — required test-suite size over (n_units, bits_per_unit)
# =============================================================================

def panel_2():
    fig, axes = plt.subplots(1, 4, figsize=(14, 3.2),
                             gridspec_kw={"width_ratios": [1, 1, 1, 1.4]})
    fig.subplots_adjust(wspace=0.42, left=0.06, right=0.97, top=0.88, bottom=0.18)

    # ── A: cycle residual accumulation ───────────────────────────────────
    ax = axes[0]
    deltas = [0.0, 0.02, 0.05, 0.10]
    traversals = np.arange(0, 30)
    for delta in deltas:
        residual = [0.3 + 3*k*delta for k in traversals]
        ax.plot(traversals, residual,
                label=f"$\\delta$={delta}", lw=1.3)
    ax.axhline(1.0, color=C1, lw=0.9, ls="--")   # cell boundary
    ax.axhline(0.0, color="#aaaaaa", lw=0.5, ls=":")
    ax.set_xlabel("traversal $k$");  ax.set_ylabel("$x_1^{(k)}$")
    ax.set_title("A   cycle residual")
    ax.legend(ncol=2)
    ax.set_ylim(-0.1, 3.5)

    # ── B: per-unit S at snapshot (unit tests see flat=β) ─────────────
    ax = axes[1]
    # Both delta=0 and delta=0.05 look identical at k=0
    units = ["$u_1$", "$u_2$", "$u_3$"]
    S_correct = [2.5, 2.5, 2.5]
    S_buggy   = [2.5, 2.5, 2.5]
    x = np.arange(3)
    w = 0.3
    ax.bar(x - w/2, S_correct, w, color=C2, alpha=0.85, label="$\\delta=0$")
    ax.bar(x + w/2, S_buggy,   w, color=C0, alpha=0.65, label="$\\delta=0.05$")
    ax.axhline(2.5, color=C1, lw=0.9, ls="--")
    ax.set_xticks(x);  ax.set_xticklabels(units)
    ax.set_ylabel("per-unit $\\mathcal{S}$")
    ax.set_title("B   unit snapshot (indistinguishable)")
    ax.legend()
    ax.set_ylim(0, 5)

    # ── C: pigeonhole — observer capacity vs pairs needed ─────────────
    ax = axes[2]
    K_R_vals   = [4, 8, 16, 32, 64, 128]
    log2_K_R   = [math.log2(k) for k in K_R_vals]
    pairs_needed = [2**(l+1) for l in log2_K_R]  # kappa(S) = log2|K_R|+1
    ax.plot(log2_K_R, K_R_vals,      color=C0, lw=1.4, label="$|K_R|$")
    ax.plot(log2_K_R, pairs_needed,  color=C1, lw=1.4, ls="--", label="pairs needed")
    ax.fill_between(log2_K_R, K_R_vals, pairs_needed,
                    where=[p > k for p, k in zip(pairs_needed, K_R_vals)],
                    color=C1, alpha=0.10)
    ax.set_xlabel("$\\log_2|K_R|$");  ax.set_ylabel("count")
    ax.set_title("C   pigeonhole bound")
    ax.legend()
    ax.set_yscale("log")

    # ── D: 3D — test-suite size over (n_units, bits) ──────────────────
    ax3d = fig.add_subplot(1, 4, 4, projection="3d", facecolor="white")
    axes[3].set_visible(False)
    n_units = np.arange(2, 14)
    bits    = np.arange(1, 5)
    NU, B   = np.meshgrid(n_units, bits)
    TS_SIZE = 2**(NU * B)   # reachable states ≈ 2^(n * bits)
    TS_LOG  = np.log2(TS_SIZE)
    ax3d.plot_surface(NU.astype(float), B.astype(float), TS_LOG,
                      cmap="plasma", rcount=30, ccount=30,
                      alpha=0.92, linewidth=0)
    ax3d.set_xlabel("units $n$", labelpad=2)
    ax3d.set_ylabel("bits/unit", labelpad=2)
    ax3d.set_zlabel("$\\log_2$(suite size)", labelpad=2)
    ax3d.set_title("D   suite size surface", pad=4)
    ax3d.view_init(elev=28, azim=-50)
    ax3d.set_facecolor("white")

    _save(fig, "panel_2.png")


# =============================================================================
# PANEL 3 — Holonomy
# A: stateless holonomy vs delta (linear)
# B: stateful — correct vs buggy accumulator holonomy
# C: trajectory divergence from C* over traversals (multiple delta)
# D: 3D — holonomy surface over (delta, traversal k)
# =============================================================================

def panel_3():
    fig, axes = plt.subplots(1, 4, figsize=(14, 3.2),
                             gridspec_kw={"width_ratios": [1, 1, 1, 1.4]})
    fig.subplots_adjust(wspace=0.42, left=0.06, right=0.97, top=0.88, bottom=0.18)

    # ── A: stateless holonomy = 3*delta ──────────────────────────────
    ax = axes[0]
    d_vals = np.linspace(0, 0.4, 200)
    hol    = 3 * d_vals
    ax.plot(d_vals, hol,  color=C0, lw=1.4)
    ax.plot(d_vals, d_vals, color="#aaaaaa", lw=0.8, ls="--")
    ax.set_xlabel("$\\delta$ (per-edge drift)")
    ax.set_ylabel("$\\mathrm{hol}(c, x)$")
    ax.set_title("A   stateless holonomy = $3\\delta$")
    ax.fill_between(d_vals, 0, hol, alpha=0.12, color=C0)

    # ── B: stateful — correct vs buggy ───────────────────────────────
    ax = axes[1]
    cases   = ["correct\n(+1.00)", "off\n+0.01", "off\n+0.1", "zero\nbug"]
    actuals = [1.0, 1.01, 1.1, 0.0]
    spec    = 1.0
    x0      = 5.0
    hols    = [abs((x0 + a) - (x0 + spec)) for a in actuals]
    colors  = [C2 if h < 1e-9 else C1 for h in hols]
    ax.bar(range(len(cases)), hols, color=colors, alpha=0.85, width=0.55)
    ax.set_xticks(range(len(cases)));  ax.set_xticklabels(cases, fontsize=6.5)
    ax.set_ylabel("$\\mathrm{hol}(c, x)$")
    ax.set_title("B   stateful holonomy")
    ax.axhline(0, color="#444444", lw=0.5)

    # ── C: x1 trajectory vs cell boundary ────────────────────────────
    ax = axes[2]
    traversals = np.arange(0, 25)
    for delta, col in zip([0.0, 0.03, 0.07, 0.15], [C2, C3, C0, C1]):
        traj = [0.3 + 3*k*delta for k in traversals]
        ax.plot(traversals, traj, color=col, lw=1.3,
                label=f"$\\delta$={delta}")
    ax.axhline(1.0, color="#333333", lw=0.9, ls="--")
    ax.set_xlabel("traversal $k$");  ax.set_ylabel("$x_1^{(k)}$")
    ax.set_title("C   trajectory exits $C^*$")
    ax.legend(fontsize=6)

    # ── D: 3D holonomy surface over (delta, k) ───────────────────────
    ax3d = fig.add_subplot(1, 4, 4, projection="3d", facecolor="white")
    axes[3].set_visible(False)
    d_arr = np.linspace(0, 0.3, 40)
    k_arr = np.arange(0, 20)
    DA, KA = np.meshgrid(d_arr, k_arr)
    HOL = 3 * DA * KA        # accumulated deviation after k traversals
    ax3d.plot_surface(DA, KA.astype(float), HOL,
                      cmap="YlOrRd", rcount=40, ccount=40,
                      alpha=0.92, linewidth=0)
    ax3d.set_xlabel("$\\delta$", labelpad=2)
    ax3d.set_ylabel("traversal $k$", labelpad=2)
    ax3d.set_zlabel("accumulated hol", labelpad=2)
    ax3d.set_title("D   holonomy surface", pad=4)
    ax3d.view_init(elev=28, azim=-45)
    ax3d.set_facecolor("white")

    _save(fig, "panel_3.png")


# =============================================================================
# PANEL 4 — Kuramoto and coordination regimes
# A: R_ens time series for K below/at/above K_c
# B: R_ens vs K sweep (onset curve)
# C: coordination friction vs R_ens (discontinuity)
# D: 3D — R_ens surface over (K, sigma_omega)
# =============================================================================

def panel_4():
    fig, axes = plt.subplots(1, 4, figsize=(14, 3.2),
                             gridspec_kw={"width_ratios": [1, 1, 1, 1.4]})
    fig.subplots_adjust(wspace=0.42, left=0.06, right=0.97, top=0.88, bottom=0.18)

    N = 20
    rng_o = random.Random(7)
    omegas = [rng_o.gauss(0, 1) for _ in range(N)]
    sigma_omega = math.sqrt(sum(w**2 for w in omegas)/N - (sum(omegas)/N)**2)
    K_c = 2 * sigma_omega / math.pi

    # ── A: R_ens time series ──────────────────────────────────────────
    ax = axes[0]
    for K_mult, col, lab in [(0.4, C1, "$K=0.4K_c$"),
                              (1.0, C3, "$K=K_c$"),
                              (2.5, C0, "$K=2.5K_c$"),
                              (5.0, C2, "$K=5K_c$")]:
        _, R_series = _kuramoto_simulate(N, K_mult*K_c, omegas,
                                          T=40, dt=0.05, seed=1)
        t_arr = np.linspace(0, 40, len(R_series))
        ax.plot(t_arr, R_series, color=col, lw=1.2, label=lab)
    ax.axhline(0.95, color="#333333", lw=0.7, ls="--")
    ax.set_xlabel("time");  ax.set_ylabel("$R_{\\mathrm{ens}}$")
    ax.set_title("A   $R_{\\mathrm{ens}}$ time series")
    ax.legend(fontsize=5.5);  ax.set_ylim(0, 1.05)

    # ── B: R_ens vs K sweep ───────────────────────────────────────────
    ax = axes[1]
    K_ratios = np.linspace(0.1, 6, 28)
    R_finals = []
    for kr in K_ratios:
        thetas_f, _ = _kuramoto_simulate(N, kr*K_c, omegas, T=80, dt=0.05, seed=2)
        R_finals.append(abs(sum(cmath.exp(1j*t) for t in thetas_f))/N)
    ax.plot(K_ratios, R_finals, color=C0, lw=1.4, marker="o",
            markersize=3, markerfacecolor=C0)
    ax.axvline(1.0, color=C1, lw=0.9, ls="--")
    ax.axhline(0.95, color="#888888", lw=0.7, ls=":")
    regime_boundaries = [0.3, 0.5, 0.8, 0.95]
    for rb in regime_boundaries:
        ax.axhline(rb, color="#cccccc", lw=0.5)
    ax.set_xlabel("$K / K_c$");  ax.set_ylabel("$R_{\\mathrm{ens}}$")
    ax.set_title("B   onset at $K_c$")
    ax.set_ylim(0, 1.05)

    # ── C: coordination friction vs R_ens ────────────────────────────
    ax = axes[2]
    R_arr  = np.linspace(0.0, 1.0, 500)
    # Friction proxy: 1 - R^2 for R < 0.95, then 0
    friction = np.where(R_arr < 0.95, (1 - R_arr**2)**2, 0.0)
    colors_f = [_regime_color(r) for r in R_arr]
    # Plot as coloured scatter
    ax.scatter(R_arr, friction, c=colors_f, s=1.5, linewidths=0)
    ax.axvline(0.95, color=C1, lw=0.9, ls="--")
    ax.set_xlabel("$R_{\\mathrm{ens}}$");  ax.set_ylabel("coordination friction $\\mathcal{F}$")
    ax.set_title("C   friction discontinuity")

    # ── D: 3D — R_ens over (K, sigma_omega) ──────────────────────────
    ax3d = fig.add_subplot(1, 4, 4, projection="3d", facecolor="white")
    axes[3].set_visible(False)
    sigma_arr = np.linspace(0.3, 2.0, 20)
    K_arr2    = np.linspace(0.1, 5.0, 20)
    SA, KA2   = np.meshgrid(sigma_arr, K_arr2)
    # Analytic mean-field approximation: R* = sqrt(max(0, 1 - K_c/K))
    # K_c = 2*sigma/pi
    KC2 = 2 * SA / math.pi
    R_MF = np.sqrt(np.maximum(0, 1 - KC2 / KA2))
    ax3d.plot_surface(SA, KA2, R_MF,
                      cmap="viridis", rcount=20, ccount=20,
                      alpha=0.92, linewidth=0)
    ax3d.set_xlabel("$\\sigma_\\omega$", labelpad=2)
    ax3d.set_ylabel("$K$", labelpad=2)
    ax3d.set_zlabel("$R_{\\mathrm{ens}}^*$", labelpad=2)
    ax3d.set_title("D   phase diagram", pad=4)
    ax3d.view_init(elev=28, azim=-50)
    ax3d.set_facecolor("white")

    _save(fig, "panel_4.png")


# =============================================================================
# PANEL 5 — Protocol: tension, decoherence, contribution scores
# A: sync tension vs aperture gap and freq difference
# B: static R_est vs mean tension (exponential decay)
# C: decoherence zone — R_est per subgraph
# D: 3D — R_est surface over (aperture_gap, freq_diff)
# (E25 contribution scores shown as inset in C)
# =============================================================================

def panel_5():
    fig, axes = plt.subplots(1, 4, figsize=(14, 3.2),
                             gridspec_kw={"width_ratios": [1, 1, 1, 1.4]})
    fig.subplots_adjust(wspace=0.42, left=0.06, right=0.97, top=0.88, bottom=0.18)

    # ── A: tension decomposition ──────────────────────────────────────
    ax = axes[0]
    gap_vals  = np.linspace(0, 2, 100)
    freq_vals = np.linspace(0, 2, 100)
    for fv, col in zip([0.0, 0.5, 1.0, 1.5], [C2, C3, C0, C1]):
        tension = gap_vals + fv
        ax.plot(gap_vals, tension, color=col, lw=1.3,
                label=f"$|\\Delta\\omega|$={fv}")
    ax.set_xlabel("aperture gap $d_X(P_i, A_j)$")
    ax.set_ylabel("sync tension $\\vartheta$")
    ax.set_title("A   tension decomposition")
    ax.legend(fontsize=6)

    # ── B: R_est vs mean tension ──────────────────────────────────────
    ax = axes[1]
    tension_arr = np.linspace(0, 4, 200)
    R_est_arr   = np.exp(-tension_arr)
    ax.plot(tension_arr, R_est_arr, color=C0, lw=1.4)
    # mark regime boundaries
    for rb, rname in [(0.3, "T"), (0.5, "AD"), (0.8, "HC"), (0.95, "C")]:
        t_b = -math.log(rb)
        ax.axvline(t_b, color="#cccccc", lw=0.7)
        ax.text(t_b + 0.05, rb + 0.03, rname, fontsize=5.5, color="#666666")
    ax.set_xlabel("mean tension $\\bar{\\vartheta}$")
    ax.set_ylabel("$R_{\\mathrm{est}}$")
    ax.set_title("B   $R_{\\mathrm{est}}$ decay")
    ax.set_ylim(0, 1.05)

    # ── C: decoherence zone bar chart + contribution scores inset ────
    ax = axes[2]
    subgraphs    = ["global", "tight\ncluster", "loose\npair", "connector"]
    tensions_sub = [
        (0.05*3 + 0.5 + 1.0) / 6,   # global mean
        0.05,                          # tight cluster
        2.5,                           # loose pair
        1.0,                           # connector edge
    ]
    R_ests = [math.exp(-t) for t in tensions_sub]
    cols   = [_regime_color(r) for r in R_ests]
    bars   = ax.bar(range(len(subgraphs)), R_ests, color=cols, alpha=0.85, width=0.55)
    ax.set_xticks(range(len(subgraphs)))
    ax.set_xticklabels(subgraphs, fontsize=6.5)
    ax.set_ylabel("$R_{\\mathrm{est}}$")
    ax.set_title("C   decoherence zones")
    ax.axhline(R_ests[0], color=C1, lw=0.9, ls="--")
    ax.set_ylim(0, 1.1)

    # inset: contribution scores (E25)
    ax_in = ax.inset_axes([0.55, 0.50, 0.42, 0.44])
    units  = ["u1","u2","u3","u4","u5"]
    dS     = [0.0, 0.0, 1/6, 0.0, 0.0]   # only u3 contributes
    icols  = [C2 if d > 1e-9 else C1 for d in dS]
    ax_in.bar(range(5), dS, color=icols, width=0.6, alpha=0.85)
    ax_in.set_xticks(range(5));  ax_in.set_xticklabels(units, fontsize=5)
    ax_in.set_ylabel("$\\delta\\mathcal{S}$", fontsize=5)
    ax_in.tick_params(labelsize=5)
    ax_in.set_title("contrib.", fontsize=5.5)

    # ── D: 3D — R_est over (aperture_gap, freq_diff) ─────────────────
    ax3d = fig.add_subplot(1, 4, 4, projection="3d", facecolor="white")
    axes[3].set_visible(False)
    gap_arr  = np.linspace(0, 3, 40)
    freq_arr = np.linspace(0, 3, 40)
    GA, FA   = np.meshgrid(gap_arr, freq_arr)
    TENSION  = GA + FA
    R_EST_3D = np.exp(-TENSION)
    ax3d.plot_surface(GA, FA, R_EST_3D,
                      cmap="RdYlGn", rcount=40, ccount=40,
                      alpha=0.92, linewidth=0)
    ax3d.set_xlabel("aperture gap", labelpad=2)
    ax3d.set_ylabel("$|\\Delta\\omega|$", labelpad=2)
    ax3d.set_zlabel("$R_{\\mathrm{est}}$", labelpad=2)
    ax3d.set_title("D   $R_{\\mathrm{est}}$ surface", pad=4)
    ax3d.view_init(elev=28, azim=-50)
    ax3d.set_facecolor("white")

    _save(fig, "panel_5.png")


# =============================================================================
# Main
# =============================================================================

if __name__ == "__main__":
    print("Generating panels...")
    panel_1()
    panel_2()
    panel_3()
    panel_4()
    panel_5()
    print(f"\nAll panels written to {OUT_DIR}/")
