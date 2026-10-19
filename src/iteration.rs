use crate::catalog::effect_catalog;
use crate::param::ParamValue;
use std::collections::BTreeMap;
use thiserror::Error;

pub const ITERATION_DELTA_TIME: f32 = 1.0_f32 / 600.0_f32;

#[derive(Clone, Debug, PartialEq)]
pub enum IterationStep {
    Read,
    Write,
    // Built-in volume I/O breaks groups like read/write, as upstream treats
    // every surface-IO step as its own iteration group.
    Read3d,
    Write3d,
    Effect {
        effect_id: String,
        params: BTreeMap<String, ParamValue>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct IterationGroup {
    pub steps: Vec<IterationStep>,
    pub iterated: bool,
    pub is_loop: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IterationFrame {
    pub frame: u32,
    pub time: f32,
    pub delta_time: f32,
}

#[derive(Debug, Error)]
pub enum IterationError {
    #[error("{0}")]
    InvalidGroup(&'static str),
    #[error(transparent)]
    Catalog(#[from] crate::catalog::CatalogError),
    #[error("iteration planner references unknown effect {0:?}")]
    UnknownEffect(String),
}

#[must_use]
pub fn is_particle_state_name(name: &str) -> bool {
    matches!(
        name,
        "global_xyz" | "global_vel" | "global_rgba" | "global_life_data"
    ) || (name.starts_with("global_") && name.ends_with("_trail"))
}

#[must_use]
pub fn wrap01(value: f32) -> f32 {
    value.rem_euclid(1.0)
}

#[must_use]
pub fn iteration_schedule(count: u32, render_time: f32) -> Vec<IterationFrame> {
    (0..count)
        .map(|frame| IterationFrame {
            frame,
            time: wrap01(
                render_time - (count.saturating_sub(1).saturating_sub(frame)) as f32 / 600.0,
            ),
            delta_time: ITERATION_DELTA_TIME,
        })
        .collect()
}

pub fn compute_iteration_groups(
    steps: &[IterationStep],
) -> Result<Vec<IterationGroup>, IterationError> {
    let catalog = effect_catalog()?;
    let mut groups = Vec::new();
    let mut particle: Option<IterationGroup> = None;
    let mut loop_steps: Option<Vec<IterationStep>> = None;

    let close_particle = |groups: &mut Vec<IterationGroup>,
                          particle: &mut Option<IterationGroup>| {
        if let Some(group) = particle.take() {
            groups.push(group);
        }
    };

    for step in steps {
        if let Some(open_loop) = loop_steps.as_mut() {
            if matches!(
                step,
                IterationStep::Read
                    | IterationStep::Write
                    | IterationStep::Read3d
                    | IterationStep::Write3d
            ) {
                return Err(IterationError::InvalidGroup(
                    "Loop iteration group cannot cross a read/write boundary",
                ));
            }
            let IterationStep::Effect { effect_id, .. } = step else {
                unreachable!()
            };
            let definition = catalog
                .effects
                .get(effect_id)
                .ok_or_else(|| IterationError::UnknownEffect(effect_id.clone()))?;
            if definition.loop_role.as_deref() == Some("begin") {
                return Err(IterationError::InvalidGroup(
                    "Nested loop iteration groups are not supported",
                ));
            }
            open_loop.push(step.clone());
            if definition.loop_role.as_deref() == Some("end") {
                groups.push(IterationGroup {
                    steps: loop_steps.take().unwrap(),
                    iterated: true,
                    is_loop: true,
                });
            }
            continue;
        }

        if matches!(
            step,
            IterationStep::Read
                | IterationStep::Write
                | IterationStep::Read3d
                | IterationStep::Write3d
        ) {
            close_particle(&mut groups, &mut particle);
            groups.push(IterationGroup {
                steps: vec![step.clone()],
                iterated: false,
                is_loop: false,
            });
            continue;
        }

        let IterationStep::Effect { effect_id, params } = step else {
            unreachable!()
        };
        let definition = catalog
            .effects
            .get(effect_id)
            .ok_or_else(|| IterationError::UnknownEffect(effect_id.clone()))?;
        match definition.loop_role.as_deref() {
            Some("end") => {
                return Err(IterationError::InvalidGroup(
                    "loopEnd has no matching loopBegin",
                ));
            }
            Some("begin") => {
                close_particle(&mut groups, &mut particle);
                loop_steps = Some(vec![step.clone()]);
                continue;
            }
            _ => {}
        }
        let declares_xyz = definition.textures.contains_key("global_xyz");
        let references_particle = definition.passes.iter().any(|pass| {
            pass.inputs
                .values()
                .chain(pass.outputs.values())
                .any(|name| is_particle_state_name(name))
        });
        if declares_xyz {
            close_particle(&mut groups, &mut particle);
            particle = Some(IterationGroup {
                steps: vec![step.clone()],
                iterated: definition.iterated,
                is_loop: false,
            });
        } else if references_particle && particle.is_some() {
            particle.as_mut().unwrap().steps.push(step.clone());
        } else if particle.is_none() {
            // The authority re-runs animated noise3d before every flow3d tick.
            let merge_with_previous = groups
                .last()
                .and_then(|previous| {
                    let IterationStep::Effect {
                        effect_id: previous_id,
                        params: previous_params,
                    } = previous.steps.first()?
                    else {
                        return None;
                    };
                    if previous.steps.len() != 1 {
                        return None;
                    }
                    Some(
                        effect_id == "filter3d/flow3d"
                            && previous_id == "synth3d/noise3d"
                            && !matches!(previous_params.get("speed"), Some(ParamValue::Int(0)))
                            && matches!(params.get("iterationCount"), Some(ParamValue::Int(count)) if *count > 0),
                    )
                })
                .unwrap_or(false);
            if merge_with_previous {
                let previous = groups.last_mut().unwrap();
                previous.steps.push(step.clone());
                previous.iterated = true;
            } else {
                groups.push(IterationGroup {
                    steps: vec![step.clone()],
                    iterated: definition.iterated,
                    is_loop: false,
                });
            }
        } else {
            close_particle(&mut groups, &mut particle);
            groups.push(IterationGroup {
                steps: vec![step.clone()],
                iterated: definition.iterated,
                is_loop: false,
            });
        }
    }
    if loop_steps.is_some() {
        return Err(IterationError::InvalidGroup(
            "loopBegin has no matching loopEnd",
        ));
    }
    close_particle(&mut groups, &mut particle);
    Ok(groups)
}
