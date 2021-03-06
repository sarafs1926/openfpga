/*
Copyright (c) 2016-2017, Robert Ou <rqou@robertou.com> and contributors
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice,
   this list of conditions and the following disclaimer.
2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
*/

use std::cmp::Ordering;
use std::collections::{HashSet, HashMap};
use std::iter::FromIterator;
use slog::Drain;

use rand::{Rng, SeedableRng, XorShiftRng};

use xc2bit::*;

use *;
use objpool::*;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum PARMCAssignment {
    MC(ObjPoolIndex<InputGraphMacrocell>),
    None,
    Banned,
}

impl slog::Value for PARMCAssignment {
    fn serialize(&self, _record: &slog::Record, key: slog::Key, serializer: &mut dyn slog::Serializer) -> slog::Result {
        match self {
            &PARMCAssignment::MC(i) => {
                serializer.emit_usize(key, i.get_raw_i())?;
                serializer.emit_str(key, "macrocell")
            },
            &PARMCAssignment::None => {
                serializer.emit_str(key, "none")
            },
            &PARMCAssignment::Banned => {
                serializer.emit_str(key, "banned")
            }
        }
    }
}

type PARFBAssignment = [(PARMCAssignment, PARMCAssignment); MCS_PER_FB];
// fb, mc, pininput?
type PARFBAssignLoc = (u32, u32, bool);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PARZIAAssignment {
    x: [[XC2ZIAInput; INPUTS_PER_ANDTERM / 2]; 2],
}

impl PARZIAAssignment {
    pub fn new() -> Self {
        Self {
            x: [[XC2ZIAInput::One; INPUTS_PER_ANDTERM / 2]; 2]
        }
    }

    pub fn get(&self, i: usize) -> XC2ZIAInput {
        self.x[i / (INPUTS_PER_ANDTERM / 2)][i % (INPUTS_PER_ANDTERM / 2)]
    }

    pub fn set(&mut self, i: usize, x: XC2ZIAInput) {
        self.x[i / (INPUTS_PER_ANDTERM / 2)][i % (INPUTS_PER_ANDTERM / 2)] = x;
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OutputGraphMacrocell {
    pub loc: Option<AssignedLocation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OutputGraphPTerm {
    pub loc: Option<AssignedLocation>,
    pub inputs_true_zia: Vec<u32>,
    pub inputs_comp_zia: Vec<u32>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct OutputGraphBufgClk {
    pub loc: Option<AssignedLocation>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct OutputGraphBufgGTS {
    pub loc: Option<AssignedLocation>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct OutputGraphBufgGSR {
    pub loc: Option<AssignedLocation>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct OutputGraph {
    pub mcs: ObjPool<OutputGraphMacrocell>,
    pub pterms: ObjPool<OutputGraphPTerm>,
    pub bufg_clks: ObjPool<OutputGraphBufgClk>,
    pub bufg_gts: ObjPool<OutputGraphBufgGTS>,
    pub bufg_gsr: ObjPool<OutputGraphBufgGSR>,
    pub zia: Vec<PARZIAAssignment>,
}

macro_rules! impl_from_ig_to_og {
    ($i_type:ident, $o_type:ident) => {
        impl std::convert::From<ObjPoolIndex<$i_type>>
            for ObjPoolIndex<$o_type> {

            fn from(x: ObjPoolIndex<$i_type>) -> Self {
                unsafe {
                    // This is fine because we are basically just turning a number into a number
                    std::mem::transmute(x)
                }
            }
        }
    }
}
impl_from_ig_to_og!(InputGraphMacrocell, OutputGraphMacrocell);
impl_from_ig_to_og!(InputGraphPTerm, OutputGraphPTerm);
impl_from_ig_to_og!(InputGraphBufgClk, OutputGraphBufgClk);
impl_from_ig_to_og!(InputGraphBufgGTS, OutputGraphBufgGTS);
impl_from_ig_to_og!(InputGraphBufgGSR, OutputGraphBufgGSR);

impl OutputGraph {
    pub fn from_input_graph(gi: &InputGraph) -> Self {
        let mut mcs = ObjPool::new();
        let mut pterms = ObjPool::new();
        let mut bufg_clks = ObjPool::new();
        let mut bufg_gts = ObjPool::new();
        let mut bufg_gsr = ObjPool::new();
        let zia = Vec::new();

        for _ in gi.mcs.iter() {
            mcs.insert(OutputGraphMacrocell {
                loc: None,
            });
        }
        for pterm in gi.pterms.iter() {
            pterms.insert(OutputGraphPTerm {
                loc: None,
                inputs_true_zia: Vec::with_capacity(pterm.inputs_true.len()),
                inputs_comp_zia: Vec::with_capacity(pterm.inputs_comp.len()),
            });
        }
        for _ in gi.bufg_clks.iter() {
            bufg_clks.insert(OutputGraphBufgClk {
                loc: None,
            });
        }
        for _ in gi.bufg_gts.iter() {
            bufg_gts.insert(OutputGraphBufgGTS {
                loc: None,
            });
        }
        for _ in gi.bufg_gsr.iter() {
            bufg_gsr.insert(OutputGraphBufgGSR {
                loc: None,
            });
        }

        Self {
            mcs,
            pterms,
            bufg_clks,
            bufg_gts,
            bufg_gsr,
            zia,
        }
    }
}

// 0 is the non-input side, 1 is the input side
fn mcs_idx_can_be_paired(g: &InputGraph,
    mc_idx_0: ObjPoolIndex<InputGraphMacrocell>, mc_idx_1: ObjPoolIndex<InputGraphMacrocell>) -> bool {

    let mc0 = g.mcs.get(mc_idx_0);
    let mc1 = g.mcs.get(mc_idx_1);
   
    mcs_can_be_paired(mc0, mc1)
}

fn mcs_can_be_paired(mc0: &InputGraphMacrocell, mc1: &InputGraphMacrocell) -> bool {
    let type_0 = mc0.get_type();
    let type_1 = mc1.get_type();
    match (type_0, type_1) {
        (InputGraphMacrocellType::BuriedComb, InputGraphMacrocellType::PinInputUnreg) |
        (InputGraphMacrocellType::BuriedComb, InputGraphMacrocellType::PinInputReg) => true,
        (InputGraphMacrocellType::BuriedReg, InputGraphMacrocellType::PinInputUnreg) => {
            // A buried reg implies that the reg output _must_ be used (if it's not, then the output must go to
            // the IO, and it wouldn't be "buried").
            assert!(mc0.reg_feedback_used);
            // If the XOR and REG feedback are both used, no room for an input
            !mc0.xor_feedback_used
        }
        _ => false,
    }
}

// First element of tuple is anything, second element can only be pin input
pub fn greedy_initial_placement(g: &mut InputGraph, go: &mut OutputGraph, device_type: XC2DeviceSpeedPackage,
    logger: &slog::Logger) -> Option<Vec<PARFBAssignment>> {

    let mut ret = Vec::new();

    // First greedily assign all of the global nets
    // TODO: Replace with BitSet when it becomes stable
    let mut gck_used = HashSet::with_capacity(NUM_BUFG_CLK);
    let mut gts_used = HashSet::with_capacity(NUM_BUFG_GTS);
    let mut gsr_used = HashSet::with_capacity(NUM_BUFG_GSR);

    // Find global buffers that have no constraint on the buffer but are fully constrained on the pin. Transfer these
    // into a constraint on the buffer.
    macro_rules! xfer_pin_loc_to_buf {
        ($g_name:ident, $cnt_name:ident, $loc_lookup:expr) => {
            for gbuf in g.$g_name.iter_mut() {
                if gbuf.requested_loc.is_some() {
                    continue;
                }

                let mc_req_loc = g.mcs.get(gbuf.input).requested_loc;
                if mc_req_loc.is_none() {
                    continue;
                }

                let mc_req_loc = mc_req_loc.unwrap();
                if mc_req_loc.i.is_some() {
                    let mut found_proper_gbuf = false;
                    for i in 0..$cnt_name {
                        let actual_mc_loc = $loc_lookup(i);

                        if mc_req_loc.fb != actual_mc_loc.0 || mc_req_loc.i.unwrap() != actual_mc_loc.1 {
                            continue;
                        }

                        // Now force the buffer to have the full location
                        found_proper_gbuf = true;
                        info!(logger, "PAR - forcing global buffer LOC due to IO pad LOC";
                            "name" => &gbuf.name,
                            "index" => i);
                        gbuf.requested_loc = Some(RequestedLocation{fb: 0, i: Some(i as u32)});
                        break;

                    }

                    if !found_proper_gbuf {
                        error!(logger, "PAR - failed to find global buffer location matching IO pad LOC constraint";
                            "io pad name" => &g.mcs.get(gbuf.input).name,
                            "fb" => mc_req_loc.fb,
                            "mc" => mc_req_loc.i.unwrap());
                        return None;
                    }
                }
            }
        }
    }
    xfer_pin_loc_to_buf!(bufg_clks, NUM_BUFG_CLK, |i| get_gck(device_type.dev, i).unwrap());
    xfer_pin_loc_to_buf!(bufg_gts, NUM_BUFG_GTS, |i| get_gts(device_type.dev, i).unwrap());
    xfer_pin_loc_to_buf!(bufg_gsr, NUM_BUFG_GSR, |_| get_gsr(device_type.dev));
    
    // Begin with assigning those that have a LOC constraint on the buffer. We know that these already have LOC
    // constraints on the pin as well.
    macro_rules! place_loc_buf {
        ($g_name:ident, $set_name:ident) => {
            for (gbuf_idx, gbuf) in g.$g_name.iter_mut_idx() {
                if let Some(RequestedLocation{i: Some(idx), ..}) = gbuf.requested_loc {
                    if $set_name.contains(&idx) {
                        error!(logger, "PAR - cannot place global buffer because site is already occupied";
                            "name" => &gbuf.name,
                            "index" => idx);
                        return None;
                    }
                    $set_name.insert(idx);

                    let gbuf_go = go.$g_name.get_mut(ObjPoolIndex::from(gbuf_idx));
                    gbuf_go.loc = Some(AssignedLocation {
                        fb: 0,
                        i: idx,
                    });

                    info!(logger, "PAR - placed global buffer (fixed)";
                        "name" => &gbuf.name,
                        "index" => idx);
                }
            }
        }
    }
    place_loc_buf!(bufg_clks, gck_used);
    place_loc_buf!(bufg_gts, gts_used);
    place_loc_buf!(bufg_gsr, gsr_used);

    // Now we assign locations to all of the remaining global buffers. Note that we checked ahead of time that there
    // aren't too many of these in use. However, it is still possible to get an unsatisfiable assignment due to
    // FB constraints on the macrocell.
    macro_rules! place_other_buf {
        ($g_name:ident, $set_name:ident, $cnt_name:ident, $loc_lookup:expr) => {
            for (gbuf_idx, gbuf) in g.$g_name.iter_mut_idx() {
                let gbuf_go = go.$g_name.get_mut(ObjPoolIndex::from(gbuf_idx));
                if gbuf_go.loc.is_some() {
                    continue;
                }

                assert!(gbuf.requested_loc.is_none());

                let mut idx = None;
                for i in 0..$cnt_name {
                    if $set_name.contains(&(i as u32)) {
                        continue;
                    }

                    let mc_req_loc = g.mcs.get(gbuf.input).requested_loc;
                    let actual_mc_loc = $loc_lookup(i);
                    assert!(mc_req_loc.is_none() || mc_req_loc.unwrap().i.is_none());
                    if mc_req_loc.is_some() && mc_req_loc.unwrap().fb != actual_mc_loc.0 {
                        debug!(logger, "PAR - skipped global buffer site because of IO pad FB constraint";
                            "name" => &gbuf.name,
                            "index" => i);
                        continue;
                    }

                    idx = Some(i as u32);
                    // Now force the MC to have the full location
                    // XXX: This can in very rare occasions cause a design that should in theory fit to no longer fit.
                    // However, we consider this to be unimportant because global nets almost always need special treatment
                    // by the HDL designer to work properly anyways.
                    g.mcs.get_mut(gbuf.input).requested_loc = Some(RequestedLocation{
                        fb: actual_mc_loc.0, i: Some(actual_mc_loc.1)});
                    info!(logger, "PAR - forcing IO pad LOC due to newly-assigned global buffer LOC";
                        "name" => &g.mcs.get(gbuf.input).name,
                        "fb" => actual_mc_loc.0,
                        "mc" => actual_mc_loc.1);
                    break;
                }

                if idx.is_none() {
                    error!(logger, "PAR - cannot place global buffer because there are no more sites";
                        "name" => &gbuf.name);
                    return None;
                }

                $set_name.insert(idx.unwrap());
                gbuf_go.loc = Some(AssignedLocation {
                    fb: 0,
                    i: idx.unwrap(),
                });

                info!(logger, "PAR - placed global buffer (free)";
                    "name" => &gbuf.name,
                    "index" => idx.unwrap());
            }
        }
    }
    place_other_buf!(bufg_clks, gck_used, NUM_BUFG_CLK, |i| get_gck(device_type.dev, i).unwrap());
    place_other_buf!(bufg_gts, gts_used, NUM_BUFG_GTS, |i| get_gts(device_type.dev, i).unwrap());
    place_other_buf!(bufg_gsr, gsr_used, NUM_BUFG_GSR, |_| get_gsr(device_type.dev));

    // Now actually assign macrocell locations
    let mut num_fbs = device_type.dev.num_fbs();
    for _ in 0..num_fbs {
        ret.push([(PARMCAssignment::None, PARMCAssignment::None); MCS_PER_FB]);
    }
    if device_type.dev == XC2Device::XC2C32 || device_type.dev == XC2Device::XC2C32A {
        num_fbs += 1;
        let x = ret.len();
        ret.push([(PARMCAssignment::Banned, PARMCAssignment::Banned); MCS_PER_FB]);
        ret[x][0] = (PARMCAssignment::Banned, PARMCAssignment::None);
    }

    // Immediately place all fully LOC'd macrocells now
    for i in g.mcs.iter_idx() {
        let mc = g.mcs.get(i);
        let is_pininput = mc.get_type().is_pininput();

        if let Some(RequestedLocation{fb, i: Some(mc_idx)}) = mc.requested_loc {
            let x = if !is_pininput {
                &mut ret[fb as usize][mc_idx as usize].0
            } else {
                &mut ret[fb as usize][mc_idx as usize].1
            };

            if *x != PARMCAssignment::None {
                error!(logger, "PAR - cannot place macrocell (fixed) because site is already occupied";
                    "name" => &mc.name,
                    "pininput" => is_pininput,
                    "fb" => fb,
                    "mc" => mc_idx);
                return None;
            }

            info!(logger, "PAR - placed macrocell (fixed)";
                "name" => &mc.name,
                "pininput" => is_pininput,
                "fb" => fb,
                "mc" => mc_idx);
            *x = PARMCAssignment::MC(i);
        }
    }

    // Check for pairing violations
    for fb_i in 0..num_fbs {
        for mc_i in 0..MCS_PER_FB {
            if let PARMCAssignment::MC(mc_idx_0) = ret[fb_i as usize][mc_i].0 {
                if let PARMCAssignment::MC(mc_idx_1) = ret[fb_i as usize][mc_i].1 {
                    if !mcs_idx_can_be_paired(g, mc_idx_0, mc_idx_1) {
                        error!(logger, "PAR - tried to force together macrocells that cannot both occupy the \
                            same location";
                            "name 1" => &g.mcs.get(mc_idx_0).name,
                            "name 2" => &g.mcs.get(mc_idx_1).name,
                            "fb" => fb_i,
                            "mc" => mc_i);
                        return None;
                    }
                }
            }
        }
    }

    // Now place macrocells that have a FB constraint but no MC constraint
    for i in g.mcs.iter_idx() {
        let mc = g.mcs.get(i);
        let is_pininput = mc.get_type().is_pininput();

        if let Some(RequestedLocation{fb, i: None}) = mc.requested_loc {
            let mut mc_i = None;
            for i in 0..MCS_PER_FB {
                if !is_pininput {
                    if ret[fb as usize][i].0 != PARMCAssignment::None {
                        continue;
                    }
                } else {
                    if ret[fb as usize][i].1 != PARMCAssignment::None {
                        continue;
                    }

                    // If this is a pin input, check if pairing is ok
                    // This logic relies on the gather_macrocells sorting.
                    if let PARMCAssignment::MC(mc_idx_0) = ret[fb as usize][i].0 {
                        if !mcs_can_be_paired(g.mcs.get(mc_idx_0), &mc) {
                            continue;
                        }
                    }
                }

                mc_i = Some(i as u32);
                break;
            }

            if mc_i.is_none() {
                error!(logger, "PAR - cannot place macrocell (FB fixed) because there are no more sites";
                    "name" => &mc.name,
                    "pininput" => is_pininput);
                return None;
            }
            let mc_idx = mc_i.unwrap();
            info!(logger, "PAR - placed macrocell (FB fixed)";
                "name" => &mc.name,
                "pininput" => is_pininput,
                "fb" => fb,
                "mc" => mc_idx);
            if !is_pininput {
                ret[fb as usize][mc_idx as usize].0 = PARMCAssignment::MC(i);
            } else {
                ret[fb as usize][mc_idx as usize].1 = PARMCAssignment::MC(i);
            }
        }
    }

    // Now place all the other macrocells
    // FIXME: Copypasta
    for i in g.mcs.iter_idx() {
        let mc = g.mcs.get(i);
        let is_pininput = mc.get_type().is_pininput();

        if mc.requested_loc.is_none() {
            let mut fbmc_i = None;
            for fb in 0..num_fbs {
                for i in 0..MCS_PER_FB {
                    if !is_pininput {
                        if ret[fb][i].0 != PARMCAssignment::None {
                            continue;
                        }
                    } else {
                        if ret[fb][i].1 != PARMCAssignment::None {
                            continue;
                        }

                        // If this is a pin input, check if pairing is ok
                        // This logic relies on the gather_macrocells sorting.
                        if let PARMCAssignment::MC(mc_idx_0) = ret[fb][i].0 {
                            if !mcs_can_be_paired(g.mcs.get(mc_idx_0), &mc) {
                                continue;
                            }
                        }
                    }

                    fbmc_i = Some((fb as u32, i as u32));
                    break;
                }

                // Need to break out again
                if fbmc_i.is_some() {
                    break;
                }
            }

            if fbmc_i.is_none() {
                error!(logger, "PAR - cannot place macrocell (free) because there are no more sites";
                    "name" => &mc.name,
                    "pininput" => is_pininput);
                return None;
            }
            let (fb, mc_idx) = fbmc_i.unwrap();
            info!(logger, "PAR - placed macrocell (free)";
                "name" => &mc.name,
                "pininput" => is_pininput,
                "fb" => fb,
                "mc" => mc_idx);
            if !is_pininput {
                ret[fb as usize][mc_idx as usize].0 = PARMCAssignment::MC(i);
            } else {
                ret[fb as usize][mc_idx as usize].1 = PARMCAssignment::MC(i);
            }
        }
    }

    // Update the "reverse" pointers
    for fb_i in 0..num_fbs {
        for mc_i in 0..MCS_PER_FB {
            if let PARMCAssignment::MC(mc_idx) = ret[fb_i as usize][mc_i].0 {
                let mc = go.mcs.get_mut(ObjPoolIndex::from(mc_idx));
                mc.loc = Some(AssignedLocation{
                    fb: fb_i as u32,
                    i: mc_i as u32,
                });
            }
            if let PARMCAssignment::MC(mc_idx) = ret[fb_i as usize][mc_i].1 {
                let mc = go.mcs.get_mut(ObjPoolIndex::from(mc_idx));
                mc.loc = Some(AssignedLocation{
                    fb: fb_i as u32,
                    i: mc_i as u32,
                });
            }
        }
    }

    Some(ret)
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum AndTermAssignmentResult {
    Success,
    FailurePtermLOCUnsatisfiable(u32),
    FailurePtermExceeded(u32),
}

pub fn try_assign_andterms(g: &InputGraph, go: &mut OutputGraph, mc_assignment: &PARFBAssignment, fb_i: u32)
    -> AndTermAssignmentResult {

    // FIXME: Too big for default to work
    let mut ret = [
        HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(),
        HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(),
        HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(),
        HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(),
        HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(),
        HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(),
        HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(),
        HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(),
    ];

    // This is a collection of p-terms that have some restrictions on where they can be placed (either because the
    // p-term is used for some special function or because there is a LOC constraint on it). The algorithm will run
    // backtracking search on these to try to find a satisfying assignment, and then afterwards it will take all of
    // the other p-terms and greedily assign them wherever there is room. We definitely don't want to put them along
    // in the backtracking search because the worst-case 54! iterations is no fun.
    let mut pterm_and_candidate_sites = Vec::new();
    let mut free_pterms = Vec::new();

    // Gather up all product terms and the locations at which they may be placed
    // Place all the special product terms
    for mc_i in 0..MCS_PER_FB {
        if let PARMCAssignment::MC(mc_g_idx) = mc_assignment[mc_i].0 {
            // FIXME: Ugly code duplication
            let this_mc = &g.mcs.get(mc_g_idx);

            if let Some(ref io_bits) = this_mc.io_bits {
                if let Some(InputGraphIOOEType::PTerm(oe_idx)) = io_bits.oe {
                    // This goes into PTB or CTE
                    let ptb_idx = get_ptb(mc_i as u32);
                    pterm_and_candidate_sites.push((oe_idx, vec![ptb_idx, CTE]));
                }
            }

            if let Some(ref xor_bits) = this_mc.xor_bits {
                if let Some(ptc_node_idx) = xor_bits.andterm_input {
                    // This goes into PTC
                    let ptc_idx = get_ptc(mc_i as u32);
                    pterm_and_candidate_sites.push((ptc_node_idx, vec![ptc_idx]));
                }
            }

            if let Some(ref reg_bits) = this_mc.reg_bits {
                if let Some(ptc_node_idx) = reg_bits.ce_input {
                    // This goes into PTC
                    let ptc_idx = get_ptc(mc_i as u32);
                    pterm_and_candidate_sites.push((ptc_node_idx, vec![ptc_idx]));
                }

                if let InputGraphRegClockType::PTerm(clk_node_idx) = reg_bits.clk_input {
                    // This goes into PTC or CTC
                    let ptc_idx = get_ptc(mc_i as u32);
                    pterm_and_candidate_sites.push((clk_node_idx, vec![ptc_idx, CTC]));
                }

                if let Some(InputGraphRegRSType::PTerm(set_node_idx)) = reg_bits.set_input {
                    // This goes into PTA or CTS
                    let pta_idx = get_pta(mc_i as u32);
                    pterm_and_candidate_sites.push((set_node_idx, vec![pta_idx, CTS]));
                }

                if let Some(InputGraphRegRSType::PTerm(reset_node_idx)) = reg_bits.reset_input {
                    // This goes into PTA or CTR
                    let pta_idx = get_pta(mc_i as u32);
                    pterm_and_candidate_sites.push((reset_node_idx, vec![pta_idx, CTR]));
                }
            }
        }
    }

    // Do a pass checking if the LOC constraints are satisfiable
    // Note that we checked if the FB matches already
    let mut loc_unsatisfiable = 0;
    for &mut (pt_idx, ref mut cand_locs) in pterm_and_candidate_sites.iter_mut() {
        let pt = g.pterms.get(pt_idx);
        if let Some(RequestedLocation{i: Some(loc_i), ..}) = pt.requested_loc {
            let mut found = false;
            for &cand_i in cand_locs.iter() {
                if cand_i == loc_i {
                    found = true;
                    break;
                }
            }

            // XXX this LOC handling is a bit busted
            if !found {
                loc_unsatisfiable += 1;
            } else {
                *cand_locs = vec![loc_i];
            }
        }
    }

    if loc_unsatisfiable > 0 {
        return AndTermAssignmentResult::FailurePtermLOCUnsatisfiable(loc_unsatisfiable);
    }

    // Finally, gather all of the remaining p-terms
    for mc_i in 0..MCS_PER_FB {
        if let PARMCAssignment::MC(mc_g_idx) = mc_assignment[mc_i].0 {
            let this_mc = &g.mcs.get(mc_g_idx);

            if let Some(ref xor_bits) = this_mc.xor_bits {
                for &pt_idx in &xor_bits.orterm_inputs {
                    let pt = g.pterms.get(pt_idx);

                    if let Some(RequestedLocation{i: Some(loc_i), ..}) = pt.requested_loc {
                        pterm_and_candidate_sites.push((pt_idx, vec![loc_i]));
                    } else {
                        free_pterms.push(pt_idx);
                    }
                }
            }
        }
    }

    // Actually do the search to assign P-terms
    // TODO: MRV/LCV?
    let mut most_placed = 0;
    fn backtrack_inner(g: &InputGraph, most_placed: &mut u32,
        ret: &mut [HashSet<ObjPoolIndex<InputGraphPTerm>>; ANDTERMS_PER_FB],
        candidate_sites: &[(ObjPoolIndex<InputGraphPTerm>, Vec<u32>)],
        working_on_idx: usize) -> bool {

        if working_on_idx == candidate_sites.len() {
            // Complete assignment, we are done
            return true;
        }
        let (pt_idx, ref candidate_sites_for_this_input) = candidate_sites[working_on_idx];
        let pt = g.pterms.get(pt_idx);
        for &candidate_pt_i in candidate_sites_for_this_input {
            if ret[candidate_pt_i as usize].is_empty() || (g.pterms.get(*ret[candidate_pt_i as usize].iter().next().unwrap()) == pt) {
                // It is possible to assign to this site
                let x = ret[candidate_pt_i as usize].insert(pt_idx);
                assert!(x);
                *most_placed = working_on_idx as u32 + 1;
                if backtrack_inner(g, most_placed, ret, candidate_sites, working_on_idx + 1) {
                    return true;
                }
                let x = ret[candidate_pt_i as usize].remove(&pt_idx);
                assert!(x);
            }
        }
        return false;
    };

    if !backtrack_inner(g, &mut most_placed, &mut ret, &pterm_and_candidate_sites, 0) {
        return AndTermAssignmentResult::FailurePtermExceeded(
            (pterm_and_candidate_sites.len() + free_pterms.len()) as u32 - most_placed);
    }

    // The backtracking search is completed. Greedily assign everything that is left.
    for &pt_idx in &free_pterms {
        let pt = g.pterms.get(pt_idx);
        let mut found = false;
        for candidate_pt_i in 0..ANDTERMS_PER_FB {
            if ret[candidate_pt_i].is_empty() || (g.pterms.get(*ret[candidate_pt_i].iter().next().unwrap()) == pt) {
                // It is possible to assign to this site
                let x = ret[candidate_pt_i].insert(pt_idx);
                assert!(x);
                most_placed += 1;
                found = true;
                break;
            }
        }

        if !found {
            // XXX really need to check if these scores follow the rules
            return AndTermAssignmentResult::FailurePtermExceeded(
                (pterm_and_candidate_sites.len() + free_pterms.len()) as u32 - most_placed);
        }
    }

    // If we got here, everything is assigned. Update the "reverse" pointers
    for pterm_i in 0..ANDTERMS_PER_FB {
        for &pt_g_idx in ret[pterm_i].iter() {
            let pterm_go = go.pterms.get_mut(ObjPoolIndex::from(pt_g_idx));
            pterm_go.loc = Some(AssignedLocation{
                fb: fb_i,
                i: pterm_i as u32,
            });
        }
    }

    AndTermAssignmentResult::Success
}

pub enum ZIAAssignmentResult {
    Success(PARZIAAssignment),
    FailureTooManyInputs(u32),
    FailureUnroutable(u32),
}

pub fn try_assign_zia(g: &InputGraph, go: &mut OutputGraph, mc_assignment: &PARFBAssignment,
    device_type: XC2DeviceSpeedPackage) -> ZIAAssignmentResult {

    let mut ret_zia = PARZIAAssignment::new();
    let mut input_to_row_map = HashMap::new();

    // Collect the p-terms that will be used by this FB
    let mut collected_pterms = Vec::new();
    for mc_i in 0..MCS_PER_FB {
        if let PARMCAssignment::MC(mc_g_idx) = mc_assignment[mc_i].0 {
            // FIXME: Ugly code duplication
            let this_mc = &g.mcs.get(mc_g_idx);

            if let Some(ref io_bits) = this_mc.io_bits {
                if let Some(InputGraphIOOEType::PTerm(oe_idx)) = io_bits.oe {
                    collected_pterms.push(oe_idx);
                }
            }

            if let Some(ref xor_bits) = this_mc.xor_bits {
                if let Some(ptc_node_idx) = xor_bits.andterm_input {
                    collected_pterms.push(ptc_node_idx);
                }

                for &andterm_node_idx in &xor_bits.orterm_inputs {
                    collected_pterms.push(andterm_node_idx);
                }
            }

            if let Some(ref reg_bits) = this_mc.reg_bits {
                if let Some(ptc_node_idx) = reg_bits.ce_input {
                    collected_pterms.push(ptc_node_idx);
                }

                if let InputGraphRegClockType::PTerm(clk_node_idx) = reg_bits.clk_input {
                    collected_pterms.push(clk_node_idx);
                }

                if let Some(InputGraphRegRSType::PTerm(set_node_idx)) = reg_bits.set_input {
                    collected_pterms.push(set_node_idx);
                }

                if let Some(InputGraphRegRSType::PTerm(reset_node_idx)) = reg_bits.reset_input {
                    collected_pterms.push(reset_node_idx);
                }
            }
        }
    }

    // Collect the inputs that need to go into this FB
    let mut collected_inputs_vec = Vec::new();
    let mut collected_inputs_set = HashSet::new();
    for &pt_idx in &collected_pterms {
        let andterm_node = g.pterms.get(pt_idx);
        for &input_net in &andterm_node.inputs_true {
            if !collected_inputs_set.contains(&input_net) {
                collected_inputs_set.insert(input_net);
                collected_inputs_vec.push(input_net);
            }
        }
        for &input_net in &andterm_node.inputs_comp {
            if !collected_inputs_set.contains(&input_net) {
                collected_inputs_set.insert(input_net);
                collected_inputs_vec.push(input_net);
            }
        }
    }

    // Must have few enough results
    if collected_inputs_vec.len() > 40 {
        return ZIAAssignmentResult::FailureTooManyInputs(collected_inputs_vec.len() as u32 - 40)
    }

    // Find candidate sites
    let candidate_sites = collected_inputs_vec.iter().map(|input| {
        let input_obj = g.mcs.get(input.1);
        let input_obj_go = go.mcs.get(ObjPoolIndex::from(input.1));
        let fb = input_obj_go.loc.unwrap().fb;
        let mc = input_obj_go.loc.unwrap().i;
        let need_to_use_ibuf_zia_path = input.0 == InputGraphPTermInputType::Reg && input_obj.xor_feedback_used;

        // What input do we actually want?
        let choice = match input.0 {
            InputGraphPTermInputType::Pin => {
                if (device_type.dev == XC2Device::XC2C32 || device_type.dev == XC2Device::XC2C32A)
                    && fb == 2 && mc == 0 {

                    XC2ZIAInput::DedicatedInput
                } else {
                    XC2ZIAInput::IBuf{ibuf:
                        fb_mc_num_to_iob_num(device_type.dev, fb as u32, mc as u32).unwrap() as u16}
                }
            },
            InputGraphPTermInputType::Xor => {
                XC2ZIAInput::Macrocell{fb: fb as u8, mc: mc as u8}
            },
            InputGraphPTermInputType::Reg => {
                // If we're forced to go via the IO path, or if it's a direct-path registered input
                if need_to_use_ibuf_zia_path || input_obj.get_type() == InputGraphMacrocellType::PinInputReg {
                    XC2ZIAInput::IBuf{ibuf:
                        fb_mc_num_to_iob_num(device_type.dev, fb as u32, mc as u32).unwrap() as u16}
                } else {
                    XC2ZIAInput::Macrocell{fb: fb as u8, mc: mc as u8}
                }
            },
        };

        // Actually search the ZIA table for it
        let mut candidate_sites_for_this_input = Vec::new();
        for zia_i in 0..INPUTS_PER_ANDTERM {
            let row = zia_table_get_row(device_type.dev, zia_i);
            for &x in row {
                if x == choice {
                    candidate_sites_for_this_input.push(zia_i);
                }
            }
        }

        (*input, choice, candidate_sites_for_this_input)
    }).collect::<Vec<_>>();

    // Actually do the search to assign ZIA rows
    let mut most_routed = 0;
    fn backtrack_inner(most_routed: &mut u32, ret: &mut PARZIAAssignment,
        candidate_sites: &[(InputGraphPTermInput, XC2ZIAInput, Vec<usize>)],
        working_on_idx: usize,
        input_to_row_map: &mut HashMap<InputGraphPTermInput, u32>) -> bool {

        if working_on_idx == candidate_sites.len() {
            // Complete assignment, we are done
            return true;
        }
        let (input, choice, ref candidate_sites_for_this_input) = candidate_sites[working_on_idx];
        for &candidate_zia_row in candidate_sites_for_this_input {
            if ret.get(candidate_zia_row) == XC2ZIAInput::One {
                // It is possible to assign to this site
                ret.set(candidate_zia_row, choice);
                input_to_row_map.insert(input, candidate_zia_row as u32);
                *most_routed = working_on_idx as u32 + 1;
                if backtrack_inner(most_routed, ret, candidate_sites, working_on_idx + 1, input_to_row_map) {
                    return true;
                }
                ret.set(candidate_zia_row, XC2ZIAInput::One);
                input_to_row_map.remove(&input);
            }
        }
        return false;
    };

    if !backtrack_inner(&mut most_routed, &mut ret_zia, &candidate_sites, 0, &mut input_to_row_map) {
        return ZIAAssignmentResult::FailureUnroutable(candidate_sites.len() as u32 - most_routed);
    }

    // Now we search through all the inputs and record which row they go in
    for &pt_idx in &collected_pterms {
        let andterm_node = g.pterms.get(pt_idx);
        let andterm_node_go = go.pterms.get_mut(ObjPoolIndex::from(pt_idx));
        andterm_node_go.inputs_true_zia.clear();
        andterm_node_go.inputs_comp_zia.clear();
        for input_net in &andterm_node.inputs_true {
            andterm_node_go.inputs_true_zia.push(*input_to_row_map.get(input_net).unwrap());
        }
        for input_net in &andterm_node.inputs_comp {
            andterm_node_go.inputs_comp_zia.push(*input_to_row_map.get(input_net).unwrap());
        }
    }

    ZIAAssignmentResult::Success(ret_zia)
}

enum FBAssignmentResultInner {
    Success(PARZIAAssignment),
    // badness
    Failure(u32),
}

fn try_assign_fb_inner(g: &InputGraph, go: &mut OutputGraph, mc_assignments: &[PARFBAssignment], fb_i: u32,
    device_type: XC2DeviceSpeedPackage) -> FBAssignmentResultInner {

    let mut failing_score = 0;
    // TODO: Weight factors?

    // Can we even assign p-terms?
    let pterm_assign_result = try_assign_andterms(g, go, &mc_assignments[fb_i as usize], fb_i);
    let zia_assign_result = try_assign_zia(g, go, &mc_assignments[fb_i as usize], device_type);

    if pterm_assign_result == AndTermAssignmentResult::Success {
        if let ZIAAssignmentResult::Success(zia_assignment) = zia_assign_result {
            return FBAssignmentResultInner::Success(zia_assignment);
        }
    }

    match pterm_assign_result {
        AndTermAssignmentResult::FailurePtermExceeded(x) => {
            failing_score += x;
        },
        AndTermAssignmentResult::FailurePtermLOCUnsatisfiable(x) => {
            failing_score += x;
        },
        AndTermAssignmentResult::Success => {},
    }

    match zia_assign_result {
        ZIAAssignmentResult::FailureTooManyInputs(x) => {
            failing_score += x;
        },
        ZIAAssignmentResult::FailureUnroutable(x) => {
            failing_score += x;
        },
        ZIAAssignmentResult::Success(_) => {},
    }

    FBAssignmentResultInner::Failure(failing_score)
}

pub fn try_assign_fb(g: &InputGraph, go: &mut OutputGraph, mc_assignments: &[PARFBAssignment], fb_i: u32,
    constraint_violations: &mut HashMap<PARFBAssignLoc, u32>, device_type: XC2DeviceSpeedPackage)
    -> Option<PARZIAAssignment> {

    let initial_assign_result = try_assign_fb_inner(g, go, mc_assignments, fb_i, device_type);

    // Check for pairing violations
    // TODO: Fix copypasta
    // XXX: Explain why this doesn't need the "remove and try again" logic?
    for mc_i in 0..MCS_PER_FB {
        if let PARMCAssignment::MC(mc_idx_0) = mc_assignments[fb_i as usize][mc_i].0 {
            if let PARMCAssignment::MC(mc_idx_1) = mc_assignments[fb_i as usize][mc_i].1 {
                if !mcs_idx_can_be_paired(g, mc_idx_0, mc_idx_1) {
                    let loc_0 = g.mcs.get(mc_idx_0).requested_loc;
                    let loc_1 = g.mcs.get(mc_idx_1).requested_loc;

                    // If not fully-LOCed, then add it as a conflict
                    if !(loc_0.is_some() && loc_0.unwrap().i.is_some()) {
                        let x = constraint_violations.insert((fb_i, mc_i as u32, false), 1);
                        assert!(x.is_none());
                    }
                    if !(loc_1.is_some() && loc_1.unwrap().i.is_some()) {
                        let x = constraint_violations.insert((fb_i, mc_i as u32, true), 1);
                        assert!(x.is_none());
                    }
                }
            }
        }
    }

    match initial_assign_result {
        FBAssignmentResultInner::Success(x) => Some(x),
        FBAssignmentResultInner::Failure(base_failing_score) => {
            // Not a success. Delete one macrocell at a time and see what happens.

            // XXX We only need this copy for the macrocell assignments. Inefficient
            let mut dummy_go = go.clone();
            let mut new_mc_assign = mc_assignments.to_owned();

            for mc_i in 0..MCS_PER_FB {
                let old_assign = new_mc_assign[fb_i as usize][mc_i].0;
                if let PARMCAssignment::MC(mc_idx) = old_assign {
                    let loc = g.mcs.get(mc_idx).requested_loc;
                    // If fully-LOCed, then we cannot move this, so don't even try deleting it or scoring it.
                    if loc.is_some() && loc.unwrap().i.is_some() {
                        continue;
                    }

                    new_mc_assign[fb_i as usize][mc_i].0 = PARMCAssignment::None;
                    let new_failing_score = match try_assign_fb_inner(g, &mut dummy_go, &new_mc_assign, fb_i,
                        device_type) {

                        FBAssignmentResultInner::Success(_) => 0,
                        FBAssignmentResultInner::Failure(x) => x,
                    };

                    if new_failing_score > base_failing_score {
                        panic!("scores are borked");
                    }

                    if base_failing_score - new_failing_score > 0 {
                        // Deleting this thing made the score better (as opposed to no change)
                        let my_loc = (fb_i, mc_i as u32, false);
                        let old_score = if constraint_violations.contains_key(&my_loc) {
                            *constraint_violations.get(&my_loc).unwrap()
                        } else { 0 };
                        let new_score = (base_failing_score - new_failing_score) as u32;
                        constraint_violations.insert(my_loc, old_score + new_score);
                    }
                    new_mc_assign[fb_i as usize][mc_i].0 = old_assign;
                }
            }

            None
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum PARSanityResult {
    Ok,
    FailurePTCNeverSatisfiable,
    FailureGlobalNetWrongLoc,
    FailureTooManyMCs,
    FailureTooManyPTerms,
    FailureTooManyBufgClk,
    FailureTooManyBufgGTS,
    FailureTooManyBufgGSR,
}

// FIXME: What happens in netlist.rs and what happens here?
pub fn do_par_sanity_check(g: &mut InputGraph, device_type: XC2DeviceSpeedPackage, logger: &slog::Logger)
    -> PARSanityResult {

    // Check if everything fits in the device

    let num_fbs = device_type.dev.num_fbs();

    if g.mcs.len() > 2 * (num_fbs * MCS_PER_FB) {
        // Note that this is a conservative fail-early check. It is incomplete because it doesn't account for
        // which macrocells can actually be paired together or which buried sites (in larger devices) can be used.
        error!(logger, "PAR (sanity) - too many total macrocells. This can never fit.";
            "num mcs" => g.mcs.len(),
            "max mcs" => 2 * (num_fbs * MCS_PER_FB));
        return PARSanityResult::FailureTooManyMCs;
    }
    info!(logger, "PAR - total macrocell count"; "num mcs" => g.mcs.len());

    let pterms_set: HashSet<InputGraphPTerm> = HashSet::from_iter(g.pterms.iter().cloned());
    if pterms_set.len() > num_fbs * ANDTERMS_PER_FB {
        error!(logger, "PAR (sanity) - too many total P-terms. This can never fit.";
            "num p-terms" => pterms_set.len(),
            "max p-terms" => num_fbs * ANDTERMS_PER_FB);
        return PARSanityResult::FailureTooManyPTerms;
    }
    info!(logger, "PAR - total unique pterm count"; "num pterms" => pterms_set.len());

    if g.bufg_clks.len() > NUM_BUFG_CLK {
        error!(logger, "PAR (sanity) - too many total BUFGs. This can never fit.";
            "num BUFGs" => g.bufg_clks.len(),
            "max BUFGs" => NUM_BUFG_CLK);
        return PARSanityResult::FailureTooManyBufgClk;
    }
    info!(logger, "PAR - total bufg count"; "num bufg" => g.bufg_clks.len());
    if g.bufg_gts.len() > NUM_BUFG_GTS {
        error!(logger, "PAR (sanity) - too many total BUFGTSs. This can never fit.";
            "num BUFGTSs" => g.bufg_gts.len(),
            "max BUFGTSs" => NUM_BUFG_GTS);
        return PARSanityResult::FailureTooManyBufgGTS;
    }
    info!(logger, "PAR - total bufgts count"; "num bufgts" => g.bufg_gts.len());
    if g.bufg_gsr.len() > NUM_BUFG_GSR {
        error!(logger, "PAR (sanity) - too many total BUFGSRs. This can never fit.";
            "num BUFGSRs" => g.bufg_gsr.len(),
            "max BUFGSRs" => NUM_BUFG_GSR);
        return PARSanityResult::FailureTooManyBufgGSR;
    }
    info!(logger, "PAR - total bufgsr count"; "num bufgsr" => g.bufg_gsr.len());

    // Check for impossible-to-satisfy PTC usage
    for mc in g.mcs.iter() {
        if let Some(ref reg_bits) = mc.reg_bits {
            if let Some(oe_node_idx) = reg_bits.ce_input {
                if let Some(ref xor_bits) = mc.xor_bits {
                    if let Some(xor_ptc_node_idx) = xor_bits.andterm_input {
                        if g.pterms.get(oe_node_idx) != g.pterms.get(xor_ptc_node_idx) {
                            error!(logger, "PAR (sanity) - OE and PTC fast path both used, but they're different";
                                "name" => &mc.name);
                            return PARSanityResult::FailurePTCNeverSatisfiable;
                        }
                    }
                }
            }
        }
    }

    // Check the LOC constraints for global nets
    macro_rules! sanity_check_bufg {
        ($g_name:ident, $loc_lookup:expr) => {
            for buf in g.$g_name.iter_mut() {
                let buf_req_loc = buf.requested_loc;
                let mc_req_loc = g.mcs.get(buf.input).requested_loc;

                match (buf_req_loc, mc_req_loc) {
                    (Some(RequestedLocation{i: Some(buf_idx), ..}), Some(mc_loc)) => {
                        // Both the pin and the buffer have a preference for where to be.

                        // These two need to match
                        let actual_mc_loc = $loc_lookup(buf_idx as usize);
                        if actual_mc_loc.0 != mc_loc.fb ||
                            (mc_loc.i.is_some() && mc_loc.i.unwrap() != actual_mc_loc.1) {

                            return PARSanityResult::FailureGlobalNetWrongLoc;
                        }

                        info!(logger, "PAR - forcing IO pad LOC due to global buffer LOC";
                            "name" => &g.mcs.get(buf.input).name,
                            "fb" => actual_mc_loc.0,
                            "mc" => actual_mc_loc.1);

                        // Now force the MC to have the full location
                        g.mcs.get_mut(buf.input).requested_loc = Some(RequestedLocation{
                            fb: actual_mc_loc.0, i: Some(actual_mc_loc.1)});
                    },
                    (Some(RequestedLocation{i: Some(buf_idx), ..}), None) => {
                        // There is a preference for the buffer, but no preference for the pin.

                        let actual_mc_loc = $loc_lookup(buf_idx as usize);

                        info!(logger, "PAR - forcing IO pad LOC due to global buffer LOC";
                            "name" => &g.mcs.get(buf.input).name,
                            "fb" => actual_mc_loc.0,
                            "mc" => actual_mc_loc.1);

                        // Now force the MC to have the full location
                        g.mcs.get_mut(buf.input).requested_loc = Some(RequestedLocation{
                            fb: actual_mc_loc.0, i: Some(actual_mc_loc.1)});
                    },
                    (Some(RequestedLocation{i: None, ..}), Some(_)) | (None, Some(_)) => {
                        // There is a preference for the pin, but no preference for the buffer.
                        // Do nothing for now, we can fail this in the greedy assignment step
                    },
                    _ => {},
                }
            }
        }
    }
    sanity_check_bufg!(bufg_clks, |i| get_gck(device_type.dev, i).unwrap());
    sanity_check_bufg!(bufg_gts, |i| get_gts(device_type.dev, i).unwrap());
    sanity_check_bufg!(bufg_gts, |_| get_gsr(device_type.dev));

    PARSanityResult::Ok
}

pub enum PARResult {
    Success(OutputGraph),
    FailureSanity(PARSanityResult),
    FailureIterationsExceeded,
}

// pub fn try_assign_fb(g: &InputGraph, go: &mut OutputGraph, mc_assignments: &[PARFBAssignment], fb_i: u32,
//     constraint_violations: &mut HashMap<PARFBAssignLoc, u32>) -> Option<PARZIAAssignment> {

pub fn try_assign_entire_chip(g: &InputGraph, go: &mut OutputGraph, mc_assignments: &[PARFBAssignment],
    device_type: XC2DeviceSpeedPackage) -> (Vec<Option<PARZIAAssignment>>, HashMap<PARFBAssignLoc, u32>, u32) {

    let num_fbs = mc_assignments.len();

    let mut par_results_per_fb = Vec::with_capacity(num_fbs);
    let mut placement_violations = HashMap::new();
    for fb_i in 0..num_fbs {
        let fb_assign_result = try_assign_fb(g, go, mc_assignments, fb_i as u32,
            &mut placement_violations, device_type);
        par_results_per_fb.push(fb_assign_result);
    }
    let mut placement_violations_score = 0;
    for x in placement_violations.values() {
        placement_violations_score += x;
    }

    (par_results_per_fb, placement_violations, placement_violations_score)
}

pub fn do_par<L: Into<Option<slog::Logger>>>(g: &mut InputGraph, device_type: XC2DeviceSpeedPackage,
    options: &XC2ParOptions, logger: L) -> PARResult {

    let logger = logger.into().unwrap_or(slog::Logger::root(slog_stdlog::StdLog.fuse(), o!()));

    let mut go = OutputGraph::from_input_graph(g);

    let sanity_check = do_par_sanity_check(g, device_type, &logger);
    if sanity_check != PARSanityResult::Ok {
        return PARResult::FailureSanity(sanity_check);
    }

    let mut prng: XorShiftRng = SeedableRng::from_seed(options.rng_seed);

    let macrocell_placement = greedy_initial_placement(g, &mut go, device_type, &logger);
    if macrocell_placement.is_none() {
        // XXX this is ugly
        return PARResult::FailureSanity(PARSanityResult::FailureTooManyMCs);
    }
    let mut macrocell_placement = macrocell_placement.unwrap();

    // Score whatever we got out of the greedy placement
    let mut best_placement = macrocell_placement.clone();
    let (mut best_par_results_per_fb, mut best_placement_violations, mut best_placement_violations_score) =
        try_assign_entire_chip(g, &mut go, &macrocell_placement, device_type);

    for iter_count in 0..options.max_iter {
        macrocell_placement = best_placement.clone();

        if best_placement_violations.len() == 0 {
            // It worked!
            info!(logger, "PAR - placement successfully found");
            for i in 0..device_type.dev.num_fbs() {
                let result_i = std::mem::replace(&mut best_par_results_per_fb[i], None);
                let zia = result_i.unwrap();
                go.zia.push(zia);
            }

            debug!(logger, "PAR - dumping final locations");
            for mc_idx in g.mcs.iter_idx() {
                let mc = g.mcs.get(mc_idx);
                let mc_go = go.mcs.get(ObjPoolIndex::from(mc_idx));
                let fb_i = mc_go.loc.unwrap().fb;
                let mc_i = mc_go.loc.unwrap().i;
                debug!(logger, "PAR - final assignment";
                    "type" => "macrocell",
                    "name" => mc.name.clone(),
                    "fb" => fb_i,
                    "mc" => mc_i);
            }
            for pterm_idx in g.pterms.iter_idx() {
                let pterm = g.pterms.get(pterm_idx);
                let pterm_go = go.pterms.get(ObjPoolIndex::from(pterm_idx));
                let fb_i = pterm_go.loc.unwrap().fb;
                let pt_i = pterm_go.loc.unwrap().i;
                debug!(logger, "PAR - final assignment";
                    "type" => "pterm",
                    "name" => pterm.name.clone(),
                    "fb" => fb_i,
                    "idx" => pt_i);
            }
            for idx in g.bufg_clks.iter_idx() {
                let bufg = g.bufg_clks.get(idx);
                let bufg_go = go.bufg_clks.get(ObjPoolIndex::from(idx));
                let idx = bufg_go.loc.unwrap().i;
                debug!(logger, "PAR - final assignment";
                    "type" => "bufg",
                    "name" => bufg.name.clone(),
                    "idx" => idx);
            }
            for idx in g.bufg_gts.iter_idx() {
                let bufg = g.bufg_gts.get(idx);
                let bufg_go = go.bufg_gts.get(ObjPoolIndex::from(idx));
                let idx = bufg_go.loc.unwrap().i;
                debug!(logger, "PAR - final assignment";
                    "type" => "bufgts",
                    "name" => bufg.name.clone(),
                    "idx" => idx);
            }
            for idx in g.bufg_gsr.iter_idx() {
                let bufg = g.bufg_gsr.get(idx);
                let bufg_go = go.bufg_gsr.get(ObjPoolIndex::from(idx));
                let idx = bufg_go.loc.unwrap().i;
                debug!(logger, "PAR - final assignment";
                    "type" => "bufgsr",
                    "name" => bufg.name.clone(),
                    "idx" => idx);
            }

            return PARResult::Success(go);
        }

        info!(logger, "PAR - new iteration";
            "iter" => iter_count,
            "score" => best_placement_violations_score);

        debug!(logger, "PAR - dumping current assignment");
        for (fb_i, fb) in best_placement.iter().enumerate() {
            for (mc_i, mc) in fb.iter().enumerate() {
                debug!(logger, "PAR - current assignment";
                    "fb" => fb_i,
                    "mc" => mc_i,
                    "non-pininput" => mc.0,
                    "pininput" => mc.1);
            }
        }
        for (&k, &v) in &best_placement_violations {
            debug!(logger, "PAR - current violations";
                "fb" => k.0,
                "mc" => k.1,
                "pininput" => k.2,
                "score" => v);
        }

        // Here, we need to swap some stuff around
        let mut bad_candidates = Vec::new();
        for (&k, &v) in &best_placement_violations {
            bad_candidates.push((k, v));
        }
        bad_candidates.sort_unstable_by(|a, b| {
            let &((a_fb, a_mc, a_pininput), _) = a;
            let &((b_fb, b_mc, b_pininput), _) = b;

            let ret = a_fb.cmp(&b_fb);
            if ret == Ordering::Equal {
                let ret = a_mc.cmp(&b_mc);
                if ret == Ordering::Equal {
                    // DEBUG: There cannot be any equality here
                    assert!(a_pininput != b_pininput);
                    a_pininput.cmp(&b_pininput)
                } else {
                    ret
                }
            } else {
                ret
            }
        });

        // Pick a candidate to move weighted by its badness
        let mut move_cand_rand = prng.gen_range(0, best_placement_violations_score);
        let mut move_cand_idx = 0;
        while move_cand_rand >= bad_candidates[move_cand_idx].1 {
            move_cand_rand -= bad_candidates[move_cand_idx].1;
            move_cand_idx += 1;
        }
        let ((move_fb, move_mc, move_pininput), _) = bad_candidates[move_cand_idx];
        info!(logger, "PAR - moving cell away";
            "fb" => move_fb, "mc" => move_mc, "pininput" => move_pininput);

        // Are we moving something that is constrained to a particular FB?
        let to_move_mc_idx = if !move_pininput {
            if let PARMCAssignment::MC(mc_idx) = macrocell_placement[move_fb as usize][move_mc as usize].0 {
                mc_idx
            } else {
                unreachable!();
            }
        } else {
            if let PARMCAssignment::MC(mc_idx) = macrocell_placement[move_fb as usize][move_mc as usize].1 {
                mc_idx
            } else {
                unreachable!();
            }
        };
        let to_move_req_fb = if let Some(RequestedLocation{fb, i}) = g.mcs.get(to_move_mc_idx).requested_loc {
            // Other code should never put something that is fully-LOCd into this list
            assert!(i.is_none());
            Some(fb)
        } else {
            None
        };

        // Don't want this to be a function because we don't want to borrow macrocell_placement until used
        macro_rules! xchg_macrocells {
            ($a_fb:expr, $a_mc:expr, $pininput:expr, $b_fb:expr, $b_mc:expr) => {
                let (a_assignment, b_assignment) = if !$pininput {
                    let a_assignment = macrocell_placement[$a_fb as usize][$a_mc as usize].0;
                    let b_assignment = macrocell_placement[$b_fb as usize][$b_mc as usize].0;
                    macrocell_placement[$b_fb as usize][$b_mc as usize].0 = a_assignment;
                    macrocell_placement[$a_fb as usize][$a_mc as usize].0 = b_assignment;
                    (a_assignment, b_assignment)
                } else {
                    let a_assignment = macrocell_placement[$a_fb as usize][$a_mc as usize].1;
                    let b_assignment = macrocell_placement[$b_fb as usize][$b_mc as usize].1;
                    macrocell_placement[$b_fb as usize][$b_mc as usize].1 = a_assignment;
                    macrocell_placement[$a_fb as usize][$a_mc as usize].1 = b_assignment;
                    (a_assignment, b_assignment)
                };

                // Swap the "loc" field as well
                if let PARMCAssignment::MC(mc_idx) = a_assignment {
                    go.mcs.get_mut(ObjPoolIndex::from(mc_idx)).loc = Some(AssignedLocation {
                        fb: $b_fb,
                        i: $b_mc,
                    });
                }
                if let PARMCAssignment::MC(mc_idx) = b_assignment {
                    go.mcs.get_mut(ObjPoolIndex::from(mc_idx)).loc = Some(AssignedLocation {
                        fb: $a_fb,
                        i: $a_mc,
                    });
                }
            }
        }

        // Find min-conflicts site
        let mut found_anything_better = false;
        let mut all_cand_sites = Vec::new();
        let mut new_best_placement_violations_score = best_placement_violations_score;
        for cand_fb in 0..device_type.dev.num_fbs() {
            if to_move_req_fb.is_some() && to_move_req_fb.unwrap() != cand_fb as u32 {
                continue;
            }

            for cand_mc in 0..MCS_PER_FB {
                // This site is not usable
                let cand_cur_assign = if !move_pininput {
                    macrocell_placement[cand_fb][cand_mc].0
                } else {
                    macrocell_placement[cand_fb][cand_mc].1
                };
                match cand_cur_assign {
                    PARMCAssignment::Banned => {
                        continue;
                    },
                    PARMCAssignment::MC(cand_mc_idx) => {
                        let cand_mc = g.mcs.get(cand_mc_idx);
                        if let Some(cand_mc_req_loc) = cand_mc.requested_loc {
                            // The thing we want to swap with has a LOC restriction of some kind
                            if cand_mc_req_loc.i.is_some() {
                                // The site we want to move to is completely LOC'd and can't be used.
                                continue;
                            }

                            if cand_mc_req_loc.fb != move_fb {
                                // The thing being moved can fit in this target site, but the thing in the target
                                // site can't fit in the thing being moved. This move cannot be performed.
                                // XXX is this the right way to do it? Technically the thing being moved can be further
                                // moved elsewhere?
                                continue;
                            }
                        }
                    },
                    PARMCAssignment::None => {},
                }

                debug!(logger, "PAR - cell candidate location";
                    "fb" => cand_fb, "mc" => cand_mc);
                all_cand_sites.push((cand_fb, cand_mc));

                // Swap it into this site
                xchg_macrocells!(move_fb, move_mc, move_pininput, cand_fb as u32, cand_mc as u32);

                // Score what we've got
                let (par_results_per_fb, new_placement_violations, new_placement_violations_score) =
                    try_assign_entire_chip(g, &mut go, &macrocell_placement, device_type);

                // Is it better? Remember it
                if new_placement_violations_score < new_best_placement_violations_score {
                    info!(logger, "PAR - cell candidate improves score";
                        "fb" => cand_fb, "mc" => cand_mc);
                    found_anything_better = true;
                    new_best_placement_violations_score = new_placement_violations_score;
                    best_placement = macrocell_placement.clone();
                    best_placement_violations = new_placement_violations;
                    best_par_results_per_fb = par_results_per_fb;
                    best_placement_violations_score = new_placement_violations_score;

                    // Is the score 0? We can immediately exit
                    if best_placement_violations.len() == 0 {
                        break;
                    }
                }

                // Swap it back
                xchg_macrocells!(move_fb, move_mc, move_pininput, cand_fb as u32, cand_mc as u32);
            }

            // Is the score 0? We can immediately exit
            if best_placement_violations.len() == 0 {
                break;
            }
        }

        if !found_anything_better {
            // No improvements possible. We have to do _something_, so move it somewhere random
            let (cand_fb, cand_mc) = all_cand_sites[prng.gen_range(0, all_cand_sites.len())];
            info!(logger, "PAR - cell forced move";
                "fb" => cand_fb, "mc" => cand_mc);

            // XXX DEFINITELY fix copypasta

            // Swap it into this site
            xchg_macrocells!(move_fb, move_mc, move_pininput, cand_fb as u32, cand_mc as u32);

            // Score what we've got
            let (par_results_per_fb, new_placement_violations, new_placement_violations_score) =
                try_assign_entire_chip(g, &mut go, &macrocell_placement, device_type);

            // Remember it
            best_placement = macrocell_placement;
            best_placement_violations = new_placement_violations;
            best_par_results_per_fb = par_results_per_fb;
            best_placement_violations_score = new_placement_violations_score;
        }
    }

    PARResult::FailureIterationsExceeded
}

#[cfg(test)]
mod tests {
    use super::*;

    use std;
    use std::fs::File;
    use std::io::Read;

    extern crate serde_json;

    fn run_one_reftest(input_filename: &'static str) {
        // Read original json
        let input_path = std::path::Path::new(input_filename);
        let mut input_data = Vec::new();
        File::open(&input_path).unwrap().read_to_end(&mut input_data).unwrap();
        let mut input_graph = serde_json::from_slice(&input_data).unwrap();
        // TODO
        let device_type = XC2DeviceSpeedPackage::from_str("xc2c32a-4-vq44").expect("invalid device name");
        // This is what we get
        let our_data_structure = if let PARResult::Success(y) = do_par(&mut input_graph, device_type,
            &XC2ParOptions::new(), None) {

            // Get a bitstream result
            let bitstream = produce_bitstream(device_type, &input_graph, &y);
            let mut ret = Vec::new();
            bitstream.to_jed(&mut ret).unwrap();
            ret
        } else {
            panic!("PAR failed!");
        };

        // Read reference jed
        let mut output_path = input_path.to_path_buf();
        output_path.set_extension("out");
        let mut output_data = Vec::new();
        File::open(&output_path).unwrap().read_to_end(&mut output_data).unwrap();
        let reference_data_structure = output_data;

        assert_eq!(our_data_structure, reference_data_structure);
    }

    // Include list of actual tests to run
    include!(concat!(env!("OUT_DIR"), "/par-reftests.rs"));
}

