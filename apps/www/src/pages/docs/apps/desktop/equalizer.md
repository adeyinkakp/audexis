---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Equalizer"
description: "Choose 5, 10, or 20 bands, use presets, and adjust preamp without modifying your music files."
---

# Equalizer

Open **Playback → Equalizer…** to adjust the sound of playback. Equalizer
settings affect audio output; they do not rewrite tags or permanently alter your
audio files.

Changes apply live and save automatically. There is no separate Save action.

## Enable or bypass

Use **EQ on / EQ off** to turn processing on or off. Disabling it
bypasses the effect while retaining your settings for later use.

Moving a slider or choosing a preset turns the equalizer on again.

If music sounds unexpectedly colored or distorted, bypass the equalizer first.
This helps distinguish an equalizer setting from a source-file or output-device
problem.

## Presets and manual settings

Choose a preset for a predefined curve, or select **Custom** to use your manual
settings. Presets adapt to the selected band layout.

The app stores manual settings separately from the selected preset. Choosing a
preset does not mean you must reconstruct your previous manual curve from
memory. Moving a band or preamp slider while a preset is selected copies that
preset’s curve into Custom for the current layout, then applies your adjustment.
This replaces the previous custom curve for that layout.

## Choose a band layout

Use **5 bands**, **10 bands**, or **20 bands** to choose how much control you
want. The default is 10 bands. Five bands provide broader adjustments; twenty
bands let you adjust smaller frequency regions.

| Layout   | Band frequencies                                                                                      |
| -------- | ----------------------------------------------------------------------------------------------------- |
| 5 bands  | 63, 250 Hz; 1, 4, 16 kHz                                                                              |
| 10 bands | 31, 62, 125, 250, 500 Hz; 1, 2, 4, 8, 16 kHz                                                          |
| 20 bands | 25, 35.5, 50, 71, 100, 140, 200, 280, 400, 560, 800 Hz; 1.1, 1.6, 2.2, 3.2, 4.5, 6.3, 9, 12.5, 18 kHz |

Each layout keeps its own custom band gains. Switching layouts restores those
values rather than converting your custom curve to a different number of
bands. The custom preamp is shared across layouts. Scroll horizontally if the
sliders do not all fit in the window.

- Lower bands affect bass and low-frequency weight.
- Middle bands affect much of the body of instruments and voices.
- Higher bands affect upper-frequency detail and brightness.

Manual band gain ranges from −12 dB to +12 dB. A positive value boosts that
band; a negative value reduces it. Start with small adjustments and compare at a
similar listening level.

## Preamp

Preamp adjusts the level before equalization. The manual range is −24 dB to 0
dB. Lower it when boosting bands to leave more headroom and reduce the chance of
excessive output levels.

A boost is not a repair for an audio file with missing information.
Lower-quality source material will not gain new detail simply because a
high-frequency band is raised.

## Reset a curve

Choose **Reset** to set the current layout’s custom band gains and the shared
custom preamp to 0 dB. This selects Custom and leaves the other layouts’ band
gains intact. Reset does not change whether the equalizer is on or off.

[Playback](/docs/apps/desktop/playback) · [Troubleshooting](/docs/apps/desktop/troubleshooting)
