//! CPU external-input state for the reactive (MIDI/audio) and mesh (OBJ) effects.
//!
//! Byte-for-byte port of the the JavaScript CPU port `src/runtime/external-input.js`
//! rendering-relevant subset: per-channel key velocities and gate, CC/pitch-bend/
//! pressure storage, the 24-PPQ clock counter, the packed 128x16 note-grid texture
//! the `roll` kernel samples, and the OBJ parser / mesh-texture packer the
//! `render/meshLoader`/`render/meshRender` passes consume. The module is
//! deliberately deterministic: no Web MIDI ports, no timing -- fixtures feed raw
//! message bytes through [`MidiState::handle_message`] exactly like the upstream
//! authority harness does.
//!
//! The fixture constants are byte-identical to the oracle's
//! `scripts/parity/reactive-fixtures.js` on purpose: both sides of a parity
//! comparison must consume exactly these values.

use std::collections::HashMap;

/// MIDI: channel 1 C-major triad (60/64/67, velocities 100/80/90), channel 2 low C
/// (48, velocity 64), then 24 clock pulses -- one beat at 24 PPQ.
pub const MIDI_MESSAGES: [[u8; 3]; 27] = [
    [0x90, 60, 100],
    [0x90, 64, 80],
    [0x90, 67, 90],
    [0x91, 48, 64],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
    [0xf8, 0, 0],
];

/// Mesh: a 12-triangle cube (8 vertices, 6 quad faces with per-face normals).
pub const CUBE_OBJ: &str = concat!(
    "v -0.7 -0.7 -0.7\n",
    "v 0.7 -0.7 -0.7\n",
    "v 0.7 0.7 -0.7\n",
    "v -0.7 0.7 -0.7\n",
    "v -0.7 -0.7 0.7\n",
    "v 0.7 -0.7 0.7\n",
    "v 0.7 0.7 0.7\n",
    "v -0.7 0.7 0.7\n",
    "vn 0 0 -1\n",
    "vn 0 0 1\n",
    "vn 0 -1 0\n",
    "vn 0 1 0\n",
    "vn -1 0 0\n",
    "vn 1 0 0\n",
    "f 1//1 2//1 3//1 4//1\n",
    "f 5//2 8//2 7//2 6//2\n",
    "f 1//3 5//3 6//3 2//3\n",
    "f 2//4 6//4 7//4 3//4\n",
    "f 3//5 7//5 8//5 4//5\n",
    "f 4//6 8//6 5//6 1//6\n",
);

pub const MESH_TEX_WIDTH: usize = 256;
pub const MESH_TEX_HEIGHT: usize = 256;

/// Deterministic audio fixture: 128-sample normalized waveform and spectrum.
#[must_use]
pub fn waveform_values() -> (Vec<f32>, Vec<f32>) {
    let mut waveform = vec![0.0_f32; 128];
    let mut spectrum = vec![0.0_f32; 128];
    for (index, (wave, bin)) in waveform.iter_mut().zip(spectrum.iter_mut()).enumerate() {
        let i = index as f64;
        *wave = (0.5 + 0.5 * ((2.0 * std::f64::consts::PI * 3.0 * i) / 128.0).sin()) as f32;
        *bin = (1.0 - i / 127.0).powi(2) as f32;
    }
    (waveform, spectrum)
}

#[derive(Clone, Debug)]
pub struct MidiChannelState {
    pub key: u8,
    pub velocity: u8,
    pub gate: u8,
    pub keys: [u8; 128],
    pub cc: [u8; 128],
    pub pitch_bend: u16,
    pub pressure: u8,
    pub poly_pressure: [u8; 128],
    pub program: u8,
}

impl Default for MidiChannelState {
    fn default() -> Self {
        Self {
            key: 0,
            velocity: 0,
            gate: 0,
            keys: [0; 128],
            cc: [0; 128],
            pitch_bend: 8192,
            pressure: 0,
            poly_pressure: [0; 128],
            program: 0,
        }
    }
}

impl MidiChannelState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn note_on(&mut self, key: usize, velocity: u8) {
        self.key = key as u8;
        self.velocity = velocity;
        self.gate = 1;
        self.keys[key] = velocity;
    }

    /// `key == None` mirrors the upstream all-notes-off overload.
    pub fn note_off(&mut self, key: Option<usize>) {
        self.gate = 0;
        match key {
            None => self.keys = [0; 128],
            Some(key) => {
                self.keys[key] = 0;
                self.poly_pressure[key] = 0;
            }
        }
    }

    pub fn control_change(&mut self, controller: usize, value: u8) {
        if controller > 127 || value > 127 {
            return;
        }
        self.cc[controller] = value;
        if controller == 120 || controller == 123 {
            self.gate = 0;
            self.keys = [0; 128];
        } else if controller == 121 {
            for (cc, slot) in self.cc.iter_mut().enumerate() {
                *slot = if cc == 11 { 127 } else { 0 };
            }
            self.pitch_bend = 8192;
            self.pressure = 0;
            self.poly_pressure = [0; 128];
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

#[derive(Clone, Debug)]
pub struct MidiState {
    pub channels: [MidiChannelState; 16],
    /// MIDI clock pulse count (24 PPQ).
    pub clock_count: u32,
    /// Note grid texture data: 128 keys x 16 channels x RGBA. Row order matches the
    /// upstream upload: row 0 is channel 1, R = velocity (0-1), G = gate, B = A = 0.
    pub note_grid: Vec<f32>,
}

impl Default for MidiState {
    fn default() -> Self {
        Self {
            channels: std::array::from_fn(|_| MidiChannelState::new()),
            clock_count: 0,
            note_grid: vec![0.0; 128 * 16 * 4],
        }
    }
}

impl MidiState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn channel(&self, channel: usize) -> Option<&MidiChannelState> {
        if (1..=16).contains(&channel) {
            Some(&self.channels[channel - 1])
        } else {
            None
        }
    }

    pub fn update_note_grid(&mut self) {
        for (channel, state) in self.channels.iter().enumerate() {
            let row_offset = channel * 128 * 4;
            for (k, &v) in state.keys.iter().enumerate() {
                let offset = row_offset + k * 4;
                self.note_grid[offset] = if v > 0 { f32::from(v) / 127.0 } else { 0.0 };
                self.note_grid[offset + 1] = if v > 0 { 1.0 } else { 0.0 };
                // B and A stay 0
            }
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Process a raw MIDI message: `[status, data1, data2]`. Mirrors the upstream
    /// routing for the message types that reach rendered state; unknown status
    /// bytes are ignored. Returns 0 when routed, -1 when the status byte is
    /// unhandled.
    pub fn handle_message(&mut self, data: &[u8]) -> i32 {
        if data.is_empty() {
            return -1;
        }
        let status = data[0];
        if status == 0xf8 {
            self.clock_count += 1;
            return 0;
        }
        if status == 0xff {
            self.reset();
            return 0;
        }
        let key = usize::from(data.get(1).copied().unwrap_or(u8::MAX));
        let velocity = data.get(2).copied().unwrap_or(u8::MAX);
        let channel = usize::from(status & 0x0f) + 1;
        let message_type = status & 0xf0;
        if message_type == 0xf0 {
            return -1;
        }
        if key > 127 {
            return -1;
        }
        if message_type != 0xd0 && velocity > 127 {
            return -1;
        }
        let Some(state) = self.channels.get_mut(channel - 1) else {
            return -1;
        };
        if message_type == 0x90 && velocity > 0 {
            state.note_on(key, velocity);
            return 0;
        }
        if message_type == 0x80 || (message_type == 0x90 && velocity == 0) {
            state.note_off(Some(key));
            return 0;
        }
        if message_type == 0xa0 {
            state.poly_pressure[key] = velocity;
            return 0;
        }
        if message_type == 0xb0 {
            state.control_change(key, velocity);
            return 0;
        }
        if message_type == 0xc0 {
            state.program = key as u8;
            return 0;
        }
        if message_type == 0xd0 {
            state.pressure = key as u8;
            return 0;
        }
        if message_type == 0xe0 {
            state.pitch_bend = u16::from(key as u8) | (u16::from(velocity) << 7);
            return 0;
        }
        -1
    }
}

/// Audio analysis state for the reactive synth effects. Upstream feeds the
/// pipeline 128-float waveform and spectrum arrays normalized to 0-1; fixtures
/// construct this state directly with deterministic arrays.
#[derive(Clone, Debug)]
pub struct AudioState {
    pub waveform: [f32; 128],
    pub spectrum: [f32; 128],
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            waveform: [0.0; 128],
            spectrum: [0.0; 128],
        }
    }
}

impl AudioState {
    pub fn set_waveform(&mut self, values: &[f32]) -> Result<(), String> {
        if values.len() != 128 {
            return Err("audio waveform requires exactly 128 samples".into());
        }
        self.waveform.copy_from_slice(values);
        Ok(())
    }

    pub fn set_spectrum(&mut self, values: &[f32]) -> Result<(), String> {
        if values.len() != 128 {
            return Err("audio spectrum requires exactly 128 bins".into());
        }
        self.spectrum.copy_from_slice(values);
        Ok(())
    }
}

/// Packed triangle-soup mesh data as consumed by the mesh effects: one texel per
/// vertex, position w = 1 marks a valid vertex (remaining texels keep w = 0).
#[derive(Clone, Debug, Default)]
pub struct MeshData {
    pub position_data: Vec<f32>,
    pub normal_data: Vec<f32>,
    pub uv_data: Vec<f32>,
    pub vertex_count: usize,
    pub tex_width: usize,
    pub tex_height: usize,
}

/// De-indexed triangle-soup vertex data parsed from Wavefront OBJ text.
#[derive(Clone, Debug, Default)]
pub struct ObjParse {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub uvs: Vec<f32>,
    pub vertex_count: usize,
}

/// Parse Wavefront OBJ text into de-indexed triangle-soup vertex data.
/// Port of the upstream obj-parser `parseOBJ`: fan triangulation for faces with
/// more than three vertices, reversed winding (OBJ CW to GL CCW), per-face normal
/// fallback when a vertex carries no `vn` reference.
pub fn parse_obj(obj_text: &str) -> ObjParse {
    let mut raw_positions: Vec<[f64; 3]> = Vec::new();
    let mut raw_normals: Vec<[f64; 3]> = Vec::new();
    let mut raw_uvs: Vec<[f64; 2]> = Vec::new();
    let mut positions: Vec<f64> = Vec::new();
    let mut normals: Vec<f64> = Vec::new();
    let mut uvs: Vec<f64> = Vec::new();

    for raw_line in obj_text.split('\n') {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts[0] {
            "v" => raw_positions.push([
                parse_f64(parts.get(1)),
                parse_f64(parts.get(2)),
                parse_f64(parts.get(3)),
            ]),
            "vn" => raw_normals.push([
                parse_f64(parts.get(1)),
                parse_f64(parts.get(2)),
                parse_f64(parts.get(3)),
            ]),
            "vt" => raw_uvs.push([parse_f64(parts.get(1)), parse_f64(parts.get(2))]),
            "f" => {
                let mut face_verts: Vec<(i64, i64, i64)> = Vec::new();
                for part in &parts[1..] {
                    let indices: Vec<&str> = part.split('/').collect();
                    let v_idx = indices
                        .first()
                        .and_then(|s| s.parse::<i64>().ok())
                        .unwrap_or(0)
                        - 1;
                    let vt_idx = indices
                        .get(1)
                        .filter(|s| !s.is_empty())
                        .and_then(|s| s.parse::<i64>().ok())
                        .map_or(-1, |v| v - 1);
                    let vn_idx = indices
                        .get(2)
                        .filter(|s| !s.is_empty())
                        .and_then(|s| s.parse::<i64>().ok())
                        .map_or(-1, |v| v - 1);
                    face_verts.push((v_idx, vt_idx, vn_idx));
                }
                // Fan triangulation, reversed winding: OBJ CW to OpenGL CCW.
                for i in 1..face_verts.len().saturating_sub(1) {
                    add_vertex(
                        &raw_positions,
                        &raw_normals,
                        &raw_uvs,
                        &mut positions,
                        &mut normals,
                        &mut uvs,
                        face_verts[0],
                    );
                    add_vertex(
                        &raw_positions,
                        &raw_normals,
                        &raw_uvs,
                        &mut positions,
                        &mut normals,
                        &mut uvs,
                        face_verts[i + 1],
                    );
                    add_vertex(
                        &raw_positions,
                        &raw_normals,
                        &raw_uvs,
                        &mut positions,
                        &mut normals,
                        &mut uvs,
                        face_verts[i],
                    );
                }
            }
            _ => {}
        }
    }

    // If no normals were provided, compute smooth vertex normals: exact port of the
    // upstream obj-parser computeFaceNormals (per-triangle face normals averaged by
    // position key, rounded to 1e-4 to merge duplicate vertices).
    if raw_normals.is_empty() && !positions.is_empty() {
        compute_face_normals(&mut positions, &mut normals);
    }

    ObjParse {
        vertex_count: positions.len() / 3,
        positions: positions.iter().map(|&v| v as f32).collect(),
        normals: normals.iter().map(|&v| v as f32).collect(),
        uvs: uvs.iter().map(|&v| v as f32).collect(),
    }
}

fn parse_f64(part: Option<&&str>) -> f64 {
    part.and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0)
}

#[allow(clippy::too_many_arguments)]
fn add_vertex(
    raw_positions: &[[f64; 3]],
    raw_normals: &[[f64; 3]],
    raw_uvs: &[[f64; 2]],
    positions: &mut Vec<f64>,
    normals: &mut Vec<f64>,
    uvs: &mut Vec<f64>,
    vertex: (i64, i64, i64),
) {
    let (v_idx, vt_idx, vn_idx) = vertex;
    if v_idx >= 0 && (v_idx as usize) < raw_positions.len() {
        positions.extend_from_slice(&raw_positions[v_idx as usize]);
    } else {
        positions.extend_from_slice(&[0.0, 0.0, 0.0]);
    }
    if vn_idx >= 0 && (vn_idx as usize) < raw_normals.len() {
        normals.extend_from_slice(&raw_normals[vn_idx as usize]);
    } else {
        normals.extend_from_slice(&[0.0, 0.0, 1.0]);
    }
    if vt_idx >= 0 && (vt_idx as usize) < raw_uvs.len() {
        uvs.extend_from_slice(&raw_uvs[vt_idx as usize]);
    } else {
        uvs.extend_from_slice(&[0.0, 0.0]);
    }
}

/// Pack triangle-soup mesh data into texture-sized RGBA arrays.
/// Port of the upstream obj-parser `packMeshDataForTextures`: one texel per
/// vertex, position w = 1 marks a valid vertex (remaining texels keep w = 0).
/// texWidth/texHeight come from the upstream mesh-texture convention (256x256).
#[must_use]
pub fn pack_mesh_data_for_textures(
    positions: &[f32],
    normals: &[f32],
    uvs: &[f32],
    tex_width: usize,
    tex_height: usize,
) -> MeshData {
    let max_vertices = tex_width * tex_height;
    let vertex_count = positions.len() / 3;
    // Upstream truncates above the cap with a console warning; parity fixtures
    // stay below it.
    let used_vertices = vertex_count.min(max_vertices);
    let pixel_count = tex_width * tex_height;
    let mut position_data = vec![0.0_f32; pixel_count * 4];
    let mut normal_data = vec![0.0_f32; pixel_count * 4];
    let mut uv_data = vec![0.0_f32; pixel_count * 4];
    for i in 0..used_vertices {
        let pi = i * 4;
        let vi3 = i * 3;
        let vi2 = i * 2;
        position_data[pi] = positions[vi3];
        position_data[pi + 1] = positions[vi3 + 1];
        position_data[pi + 2] = positions[vi3 + 2];
        position_data[pi + 3] = 1.0;
        normal_data[pi] = normals[vi3];
        normal_data[pi + 1] = normals[vi3 + 1];
        normal_data[pi + 2] = normals[vi3 + 2];
        normal_data[pi + 3] = 0.0;
        uv_data[pi] = uvs[vi2];
        uv_data[pi + 1] = uvs[vi2 + 1];
        uv_data[pi + 2] = 0.0;
        uv_data[pi + 3] = 0.0;
    }
    // Remaining texels keep w = 0 (position_data is zero-initialized).
    MeshData {
        position_data,
        normal_data,
        uv_data,
        vertex_count: used_vertices,
        tex_width,
        tex_height,
    }
}

/// Smooth vertex normals for un-normalized OBJ files: exact port of the upstream
/// obj-parser computeFaceNormals (face normals from reversed-winding triangles,
/// averaged per rounded position key, threshold 1e-4).
fn compute_face_normals(positions: &mut [f64], normals: &mut [f64]) {
    let vertex_count = positions.len() / 3;
    let triangle_count = vertex_count / 3;
    let mut face_normals = vec![0.0_f64; triangle_count * 3];
    for tri in 0..triangle_count {
        let i0 = tri * 9;
        let i1 = i0 + 3;
        let i2 = i0 + 6;
        let (ax, ay, az) = (positions[i0], positions[i0 + 1], positions[i0 + 2]);
        let (bx, by, bz) = (positions[i1], positions[i1 + 1], positions[i1 + 2]);
        let (cx, cy, cz) = (positions[i2], positions[i2 + 1], positions[i2 + 2]);
        let (e1x, e1y, e1z) = (bx - ax, by - ay, bz - az);
        let (e2x, e2y, e2z) = (cx - ax, cy - ay, cz - az);
        let mut nx = e1y * e2z - e1z * e2y;
        let mut ny = e1z * e2x - e1x * e2z;
        let mut nz = e1x * e2y - e1y * e2x;
        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        if len > 0.0001 {
            nx /= len;
            ny /= len;
            nz /= len;
        } else {
            nx = 0.0;
            ny = 0.0;
            nz = 1.0;
        }
        face_normals[tri * 3] = nx;
        face_normals[tri * 3 + 1] = ny;
        face_normals[tri * 3 + 2] = nz;
    }
    let mut pos_to_normal: HashMap<String, [f64; 4]> = HashMap::new();
    let round = |v: f64| (v * 10000.0).round() / 10000.0;
    for v in 0..vertex_count {
        let (px, py, pz) = (positions[v * 3], positions[v * 3 + 1], positions[v * 3 + 2]);
        let key = format!("{},{},{}", round(px), round(py), round(pz));
        let tri_idx = v / 3;
        let acc = pos_to_normal.entry(key).or_insert([0.0, 0.0, 0.0, 0.0]);
        acc[0] += face_normals[tri_idx * 3];
        acc[1] += face_normals[tri_idx * 3 + 1];
        acc[2] += face_normals[tri_idx * 3 + 2];
        acc[3] += 1.0;
    }
    for acc in pos_to_normal.values_mut() {
        let len = (acc[0] * acc[0] + acc[1] * acc[1] + acc[2] * acc[2]).sqrt();
        if len > 0.0001 {
            acc[0] /= len;
            acc[1] /= len;
            acc[2] /= len;
        } else {
            acc[0] = 0.0;
            acc[1] = 0.0;
            acc[2] = 1.0;
        }
    }
    for v in 0..vertex_count {
        let (px, py, pz) = (positions[v * 3], positions[v * 3 + 1], positions[v * 3 + 2]);
        let key = format!("{},{},{}", round(px), round(py), round(pz));
        let acc = &pos_to_normal[&key];
        normals[v * 3] = acc[0];
        normals[v * 3 + 1] = acc[1];
        normals[v * 3 + 2] = acc[2];
    }
}

/// External data-texture surfaces for the reactive (MIDI) and mesh (OBJ) effects.
///
/// Data textures uploaded from JS arrays on WebGL2 place array row 0 at GL texture
/// coordinate y = 0 (bottom-left origin). The CPU surfaces store rows top-down and
/// the GLSL samplers flip the y coordinate, so a data-texture surface must store
/// the uploaded array's rows reversed for both `texture()` and `texelFetch()` to
/// read the same texel the GPU reads. The mesh triangles adapter reads the raw
/// uploaded arrays directly (GPU `texelFetch(x, y)` = data[(y * width + x) * 4],
/// no flip) via the [`MeshData`] payload.
#[must_use]
pub fn flip_rgba_rows(data: &[f32], width: usize, height: usize) -> Vec<f32> {
    let mut flipped = vec![0.0_f32; width * height * 4];
    for row in 0..height {
        let source = (height - 1 - row) * width * 4;
        flipped[row * width * 4..(row + 1) * width * 4]
            .copy_from_slice(&data[source..source + width * 4]);
    }
    flipped
}

/// The external inputs one render may bind. Fixtures are deterministic; see
/// [`midi_fixture`], [`audio_fixture`], and [`mesh_fixture`].
#[derive(Clone, Debug, Default)]
pub struct ExternalInputs {
    pub midi_state: Option<MidiState>,
    pub audio_state: Option<AudioState>,
    pub mesh_data: Option<MeshData>,
}

impl ExternalInputs {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.midi_state.is_none() && self.audio_state.is_none() && self.mesh_data.is_none()
    }
}

/// External inputs for one parity case id, or none when the case needs none.
#[must_use]
pub fn external_inputs_for_case(id: &str) -> ExternalInputs {
    let mut inputs = ExternalInputs::default();
    match id {
        "synth/roll" => inputs.midi_state = Some(midi_fixture()),
        "synth/scope" | "synth/spectrum" => inputs.audio_state = Some(audio_fixture()),
        "render/meshLoader" | "render/meshRender" => inputs.mesh_data = Some(mesh_fixture()),
        _ => {}
    }
    inputs
}

#[must_use]
pub fn midi_fixture() -> MidiState {
    let mut midi_state = MidiState::new();
    for message in &MIDI_MESSAGES {
        midi_state.handle_message(message);
    }
    midi_state.update_note_grid();
    midi_state
}

#[must_use]
pub fn audio_fixture() -> AudioState {
    let mut audio_state = AudioState::default();
    let (waveform, spectrum) = waveform_values();
    audio_state.set_waveform(&waveform).expect("128 samples");
    audio_state.set_spectrum(&spectrum).expect("128 bins");
    audio_state
}

#[must_use]
pub fn mesh_fixture() -> MeshData {
    let parsed = parse_obj(CUBE_OBJ);
    let mut packed = pack_mesh_data_for_textures(
        &parsed.positions,
        &parsed.normals,
        &parsed.uvs,
        MESH_TEX_WIDTH,
        MESH_TEX_HEIGHT,
    );
    packed.tex_width = MESH_TEX_WIDTH;
    packed.tex_height = MESH_TEX_HEIGHT;
    packed
}
