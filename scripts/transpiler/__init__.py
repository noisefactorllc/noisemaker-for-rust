"""GLSL -> Python transpiler for the noisemaker-cpu Python port.

Pure Python. Fetches shader source from the shaders.noisedeck.app CDN, parses
the GLSL, and emits native Python pixel kernels. This is build-time tooling that
regenerates src/noisemaker_cpu/bundle/.
"""
