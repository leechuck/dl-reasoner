use std::fmt;
use std::hash;
use std::collections::HashSet;
use std::iter::FromIterator;

use abox::{ABox, ABoxAxiom, ABoxAxiomType, ConceptAxiom};
use concept::{Concept, ConjunctionConcept, DisjunctionConcept, parse_concept};


pub fn parse_tbox(tbox_str: &str) -> TBox {
    debug!("Parsing TBox!");

    let tbox_str = tbox_str.trim();
    let mut tbox = TBox::new();

    for line in tbox_str.lines() {
        debug!("Parsing line: {}", line);

        if line.len() > 0 && !line.starts_with('#') {
            tbox.axioms.insert(Box::new(parse_tbox_axiom(line)));
        }
    }

    tbox
}


pub fn parse_tbox_axiom(tbox_line: &str) -> TBoxAxiom {
    let tbox_line = tbox_line.trim();
    let delimiter = if tbox_line.contains("==") { "==" } else { "->" };
    let axiom_type = if delimiter == "==" { TBoxAxiomType::Definition } else { TBoxAxiomType::Inclusion };
    let delimiter_idx = tbox_line.find(delimiter).unwrap();

    TBoxAxiom {
        axiom_type: axiom_type,
        lhs: parse_concept(&tbox_line[..delimiter_idx]).convert_to_nnf(),
        rhs: parse_concept(&tbox_line[delimiter_idx + 2..]).convert_to_nnf()
    }
}

#[derive(Debug, Clone)]
pub struct TBox {
    axioms: HashSet<Box<TBoxAxiom>>
}

impl TBox {
    pub fn new() -> TBox {
        TBox {axioms: HashSet::new()}
    }
}

impl TBox {
    pub fn expand_all_definitions(&mut self) {
        info!("Expanding TBox definitions...");
        // Expands all the definitions in such a way that we do not use
        // definitions inside definitions
        let mut definitions = self.axioms.clone().into_iter()
            .filter(|a| a.axiom_type == TBoxAxiomType::Definition)
            .collect::<Vec<Box<TBoxAxiom>>>();
        let mut definitions_updated = definitions.clone();
        let mut applied_defs = HashSet::new();

        while let Some(def) = definitions.pop() {
            applied_defs.insert(def.lhs.clone());
            // Expanding the definition in all the possible definitions
            // After that we will not have this definition anywhere except for itself
            definitions_updated = definitions_updated
                .into_iter()
                .clone()
                .map(|d| {
                    if def.lhs.to_string() == d.lhs.to_string() {
                        Box::new(*d)
                    } else {
                        Box::new(TBoxAxiom {
                            axiom_type: d.axiom_type.clone(),
                            lhs: d.lhs.clone(),
                            rhs: d.rhs.replace_concept(def.lhs.clone(), def.rhs.clone())
                        })
                    }
                })
                .collect::<Vec<Box<TBoxAxiom>>>();

            definitions = definitions_updated.clone()
                .into_iter()
                .filter(|d| {!applied_defs.contains(&d.lhs)})
                .collect();
        }
    }

    pub fn apply_definitions_to_abox(&self, abox: &mut ABox) {
        info!("Applying expanded TBox definitions to an ABox...");
        let definitions = self.axioms.clone().into_iter()
            .filter(|a| a.axiom_type == TBoxAxiomType::Definition)
            .collect::<Vec<Box<TBoxAxiom>>>();

        abox.axioms = HashSet::from_iter(abox.axioms.clone().iter().map(|a| {
            match a.axiom_type() {
                ABoxAxiomType::Relation => a.clone(),
                ABoxAxiomType::Concept => {
                    let axiom = a.downcast_ref::<ConceptAxiom>().unwrap();
                    let replaced_concept = definitions.clone()
                        .iter()
                        .fold(axiom.concept.clone(), |c, def| {
                            c.replace_concept(def.lhs.clone(), def.rhs.clone())
                        });

                    Box::new(ConceptAxiom {
                        concept: replaced_concept,
                        individual: axiom.individual.clone()
                    }) as Box<dyn ABoxAxiom>
                },
            }
        }))
    }

    pub fn apply_definitions_to_inclusions(&mut self) {
        info!("Applying expanded TBox definitions to GCIs...");
        let definitions = self.axioms.clone().into_iter()
            .filter(|a| a.axiom_type == TBoxAxiomType::Definition)
            .collect::<Vec<Box<TBoxAxiom>>>();
        let mut inclusions = self.axioms.clone().into_iter()
            .filter(|a| a.axiom_type == TBoxAxiomType::Inclusion)
            .collect::<Vec<Box<TBoxAxiom>>>();

        for inclusion in &mut inclusions {
            for def in &definitions {
                inclusion.lhs = inclusion.lhs.replace_concept(def.lhs.clone(), def.rhs.clone());
                inclusion.rhs = inclusion.rhs.replace_concept(def.lhs.clone(), def.rhs.clone());
            }
        }
    }

    pub fn aggregate_inclusions(&self) -> Option<ConjunctionConcept> {
        info!("Aggregating GCIs into a single one...");
        let inclusions = self.axioms.clone().into_iter()
            .filter(|a| a.axiom_type == TBoxAxiomType::Inclusion)
            .collect::<Vec<Box<TBoxAxiom>>>();

        if inclusions.is_empty() {
            return None;
        }

        let subconcepts = inclusions
            .into_iter()
            .map(|ta| {DisjunctionConcept {subconcepts: vec![ta.lhs.negate().convert_to_nnf(), ta.rhs]}})
            .map(|a| Box::new(a) as Box<dyn Concept>)
            .collect::<Vec<Box<dyn Concept>>>();

        Some(ConjunctionConcept {subconcepts: subconcepts})
    }
}

impl fmt::Display for TBox {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "TBox:\n  - {}", self.axioms.iter()
            .map(|a| a.to_string()).collect::<Vec<String>>().join("\n  - "))
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum TBoxAxiomType { Definition, Inclusion }

#[derive(Debug, Clone, Eq)]
pub struct TBoxAxiom {
    pub axiom_type: TBoxAxiomType,
    pub lhs: Box<dyn Concept>,
    pub rhs: Box<dyn Concept>,
}

impl fmt::Display for TBoxAxiom {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let delimiter = if self.axiom_type == TBoxAxiomType::Definition {"=="} else {"->"};
        write!(fmt, "{} {} {}", self.lhs.to_string(), delimiter, self.rhs.to_string())
    }
}

// We have to implement everyhing manually since
// damn rust cannor derive these traits without "move occurs" error
impl PartialEq for TBoxAxiom {
    fn eq(&self, other: &TBoxAxiom) -> bool {
        self.to_string() == other.to_string()
    }
}

impl hash::Hash for TBoxAxiom {
    fn hash<H: hash::Hasher>(&self, hasher: &mut H) {
        self.to_string().hash(hasher);
    }
}
