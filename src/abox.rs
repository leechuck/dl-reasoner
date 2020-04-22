/*
    ABox axioms have the following format:
    "C[x]", "r[x, y]", "(some r C)[x]", etc.
    This makes parsing easy without the loss of readability
*/
use std::string;
use common::{Individual, Relation, Concept, parse_concept};

#[derive(Debug)]
pub enum ABoxAxiom {
    Concept(ConceptAxiom),
    Relation(RelationAxiom)
}

#[derive(Debug)]
pub struct RelationAxiom {
    pub relation: Relation,
    pub lhs: Individual,
    pub rhs: Individual,
}

#[derive(Debug)]
pub struct ConceptAxiom {
    pub concept: Concept,
    pub individual: Individual
}


pub fn parse_abox(abox_str: &str) -> Vec<ABoxAxiom> {
    let abox_str = abox_str.trim();
    let mut abox_axioms = Vec::new();

    for line in abox_str.lines() {
        println!("Parsing line: {}", line);
        abox_axioms.push(parse_abox_axiom(&line))
    }

    abox_axioms
}


pub fn parse_abox_axiom(axiom_str: &str) -> ABoxAxiom {
    let axiom_str = axiom_str.trim();
    let start_idx = axiom_str.find("[").unwrap_or(0);
    let end_idx = axiom_str.find("]").unwrap_or(axiom_str.len());
    let arguments_str = &axiom_str[start_idx..end_idx + 1].trim();
    println!("arguments string: {}", arguments_str);
    let mut individuals = arguments_str
        .split(",").map(|n| (Individual {name: n.to_string()}))
        .collect::<Vec<_>>();

    if arguments_str.contains(",") {
        // This is a relation axiom
        ABoxAxiom::Relation(RelationAxiom {
            relation: Relation { name: axiom_str[..start_idx].to_string() },
            lhs: individuals.remove(0),
            rhs: individuals.remove(1),
        })
    } else {
        // This is a concept axiom
        ABoxAxiom::Concept(ConceptAxiom {
            concept: parse_concept(&axiom_str[..start_idx]),
            individual: individuals.remove(0)
        })
    }
}
