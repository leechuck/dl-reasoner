use std::fmt::Debug;
use std::clone::Clone;
use std::any::{Any, TypeId};
use std::marker::Sized;


pub trait Concept: Debug + mopa::Any + ConceptClone {
    fn convert_to_nnf(&self) -> Box<dyn Concept>;

    fn negate(&self) -> Box<dyn Concept> {
        // Box::new(NotConcept{ subconcept: Box::new(self.clone()) })
        Box::new(NotConcept{ subconcept: Box::new(self).clone_box() })
    }
}

pub trait ConceptClone {
    fn clone_box(&self) -> Box<dyn Concept>;
}

impl<T> ConceptClone for T where T: Concept + Clone {
    fn clone_box(&self) -> Box<dyn Concept> { Box::new(self.clone()) }
}

// We can now implement Clone manually by forwarding to clone_box.
impl Clone for Box<dyn Concept> {
    fn clone(&self) -> Box<dyn Concept> { self.clone_box() }
}

mopafy!(Concept);

#[derive(Debug, Clone)]
pub struct Relation { pub name: String }

#[derive(Debug, Clone)]
pub struct Individual { pub name: String }

#[derive(Debug, Clone)]
pub struct AtomicConcept { name: String }

#[derive(Debug, Clone)]
pub struct NotConcept {
    subconcept: Box<dyn Concept>
}

#[derive(Debug, Clone)]
pub struct ConjunctionConcept {
    subconcepts: Vec<Box<dyn Concept>>
}

#[derive(Debug, Clone)]
pub struct DisjunctionConcept {
    subconcepts: Vec<Box<dyn Concept>>
}

#[derive(Debug, Clone)]
pub struct OnlyConcept {
    subconcept: Box<dyn Concept>,
    relation: Relation
}

#[derive(Debug, Clone)]
pub struct SomeConcept {
    subconcept: Box<dyn Concept>,
    relation: Relation
}

impl Concept for AtomicConcept {
    fn convert_to_nnf(&self) -> Box<dyn Concept> {
        Box::new(self.clone())
    }
}

impl Concept for NotConcept {
    fn convert_to_nnf(&self) -> Box<dyn Concept> {
        if self.subconcept.is::<AtomicConcept>() {
            let subconcept = self.subconcept.downcast_ref::<AtomicConcept>().unwrap();
            Box::new(NotConcept { subconcept: Box::new(subconcept.clone()) })
        } else if self.subconcept.is::<NotConcept>() {
            let subconcept = self.subconcept.downcast_ref::<NotConcept>().unwrap();
            subconcept.subconcept.convert_to_nnf()
        } else if self.subconcept.is::<ConjunctionConcept>() {
            // not and (A B C) => or ((not A) (not B) (not C))
            let subconcept = self.subconcept.downcast_ref::<ConjunctionConcept>().unwrap();
            // Box::new(AtomicConcept { name: "123".to_string() })
            Box::new(DisjunctionConcept {
                subconcepts: subconcept.clone().subconcepts.iter()
                    .map(|c| { c.negate() })
                    .map(|c| {c.convert_to_nnf()})
                    .collect()
            })
        } else if self.subconcept.is::<DisjunctionConcept>() {
            // not [or (A B C)] => and ((not A) (not B) (not C))
            let subconcept = self.subconcept.downcast_ref::<DisjunctionConcept>().unwrap();
            Box::new(ConjunctionConcept {
                subconcepts: subconcept.clone().subconcepts.iter()
                    .map(|c| { Box::new(NotConcept{ subconcept: c.clone() }) })
                    .map(|c| {c.convert_to_nnf()})
                    .collect()
            })
        } else if self.subconcept.is::<OnlyConcept>() {
            // not [only A] => some [not A]
            let subconcept = self.subconcept.downcast_ref::<OnlyConcept>().unwrap();
            Box::new(SomeConcept {
                relation: subconcept.relation.clone(),
                subconcept: subconcept.subconcept.negate().convert_to_nnf()
            })
        } else if self.subconcept.is::<SomeConcept>() {
            // not [some A] => only [not A]
            let subconcept = self.subconcept.downcast_ref::<SomeConcept>().unwrap();
            Box::new(OnlyConcept {
                relation: subconcept.relation.clone(),
                subconcept: subconcept.subconcept.negate().convert_to_nnf()
            })
        } else {
            unimplemented!();
        }


    }
}
impl Concept for ConjunctionConcept {
    fn convert_to_nnf(&self) -> Box<dyn Concept> {
        Box::new(ConjunctionConcept {
            subconcepts: self.subconcepts.iter().map(|c| { c.convert_to_nnf() }).collect()
        })
    }
}
impl Concept for DisjunctionConcept {
    fn convert_to_nnf(&self) -> Box<dyn Concept> {
        Box::new(DisjunctionConcept {
            subconcepts: self.subconcepts.iter().map(|c| { c.convert_to_nnf() }).collect()
        })
    }
}
impl Concept for OnlyConcept {
    fn convert_to_nnf(&self) -> Box<dyn Concept> {
        Box::new(OnlyConcept {
            relation: self.relation.clone(),
            subconcept: self.subconcept.convert_to_nnf()
        })
    }
}
impl Concept for SomeConcept {
    fn convert_to_nnf(&self) -> Box<dyn Concept> {
        Box::new(SomeConcept {
            relation: self.relation.clone(),
            subconcept: self.subconcept.convert_to_nnf()
        })
    }
}


pub fn parse_concept(concept_str: &str) -> Box<dyn Concept> {
    // Parses concept or panics if the string is not a correct concept
    // let mut words = concept_str.split(' ').collect();
    let concept_str = concept_str.trim();

    println!("Parsing concept: {}", concept_str);

    if &concept_str[..1] == "(" {
        // Our concept is wrapped up into brackets "(..)"
        parse_concept(&concept_str[1..(concept_str.len() - 1)])
    } else if concept_str.len() > 3 && &concept_str[..3] == "and" {
        // println!("It is and!");
        Box::new(ConjunctionConcept { subconcepts: extract_concepts(&concept_str[3..]) })
    } else if concept_str.len() > 2 && &concept_str[..2] == "or" {
        // println!("It is or!");
        Box::new(DisjunctionConcept { subconcepts: extract_concepts(&concept_str[2..]) })
    } else if concept_str.len() > 4 && &concept_str[..4] == "only" {
        // println!("It is only!");
        Box::new(OnlyConcept {
            relation: Relation {name: concept_str.chars().nth(5).unwrap().to_string()},
            subconcept: parse_concept(&concept_str[6..])
        })
    } else if concept_str.len() > 4 && &concept_str[..4] == "some" {
        // println!("It is some!");
        Box::new(SomeConcept {
            relation: Relation {name: concept_str.chars().nth(5).unwrap().to_string()},
            subconcept: parse_concept(&concept_str[6..])
        })
    } else if concept_str.len() > 3 && &concept_str[..3] == "not" {
        println!("It is not!");
        Box::new(NotConcept { subconcept: parse_concept(&concept_str[3..]) })
    } else {
        println!("It is an atomic concept!");
        // This is an Atomic Concept!
        Box::new(AtomicConcept { name: concept_str.to_string() })
    }
}


fn extract_concepts(concepts_str: &str) -> Vec<Box<dyn Concept>> {
    // Takes a concepts string, seperated by whitespace and wrapped up in brackets,
    // parses them individually and returns a vector of concepts.
    let concepts_str = concepts_str.trim();
    println!("Extractinc concepts: {}", concepts_str);
    let mut concepts: Vec<Box<dyn Concept>> = Vec::new();
    let mut curr_depth = 0;
    let mut curr_concept_start_idx = 0;
    let mut i = 0;

    while i < concepts_str.len() {
        if &concepts_str[i..i + 1] == "(" {
            curr_depth += 1; // Going a level deeper
        } else if &concepts_str[i..i + 1] == ")" {
            curr_depth -= 1; // Going a level out
        }

        if curr_depth == 0 {
            println!("Found concept: {}", &concepts_str[curr_concept_start_idx .. i + 1]);
            concepts.push(parse_concept(&concepts_str[curr_concept_start_idx .. i + 1]));
            curr_concept_start_idx = i + 1; // Next concept starts on the next character
            i += 1;
        }

        i += 1;
    }
    // for (i, c) in concepts_str.chars().enumerate() {
    //     if c == '(' {
    //         curr_depth += 1; // Going a level deeper
    //     } else if c == ')' {
    //         curr_depth -= 1; // Going a level out
    //     }

    //     if curr_depth == 0 {
    //         println!("Found concept: {}", &concepts_str[curr_concept_start_idx..i+1]);
    //         concepts.push(parse_concept(&concepts_str[curr_concept_start_idx..i+1]));
    //         curr_concept_start_idx = i; // Next concept starts on the next character
    //     }
    // }

    debug_assert!(concepts.len() > 0);

    concepts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_concepts() {
        assert_eq!(extract_concepts("C"), vec![AtomicConcept {name: "C"}]);
    }
}