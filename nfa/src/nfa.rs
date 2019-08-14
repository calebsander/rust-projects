use std::vec::IntoIter;
use bit_vector::BitVector;

type NodeIndex = usize;
enum NodeType {
	Terminal,
	All { next: NodeIndex },
	Exact { c: char, next: NodeIndex },
}
impl NodeType {
	fn get_transition(&self, match_char: char) -> Option<NodeIndex> {
		use NodeType::*;
		match self {
			Terminal => None,
			All { next } => Some(*next),
			Exact { c, next } => if match_char == *c { Some(*next) } else { None },
		}
	}
}

struct Node {
	node: NodeType,
	epsilon_transitions: Vec<NodeIndex>,
}
struct DistinguishedNodes {
	start: NodeIndex,
	accept: NodeIndex,
}

struct NodeSet {
	values: Vec<NodeIndex>,
	present: BitVector,
}
impl NodeSet {
	fn new(size: usize) -> Self {
		let mut present = BitVector::with_capacity(size);
		for _ in 0..size { present.push(false) }
		NodeSet { values: vec![], present }
	}

	fn contains(&self, index: NodeIndex) -> bool {
		self.present.get(index).unwrap()
	}
	fn insert(&mut self, index: NodeIndex) -> bool {
		let contains = self.contains(index);
		if !contains {
			self.values.push(index);
			self.present.set(index, true).unwrap();
		}
		contains
	}
	fn is_empty(&self) -> bool {
		self.values.is_empty()
	}
}
impl IntoIterator for NodeSet {
	type Item = NodeIndex;
	type IntoIter = IntoIter<NodeIndex>;

	fn into_iter(self) -> Self::IntoIter {
		self.values.into_iter()
	}
}

pub struct NFA {
	nodes: Vec<Node>,
	distinguished_nodes: DistinguishedNodes,
}
impl NFA {
	fn add_reachable(&self, nodes: &mut NodeSet, start: NodeIndex) {
		if nodes.insert(start) {
			for node in &self.nodes[start].epsilon_transitions {
				self.add_reachable(nodes, *node)
			}
		}
	}
	pub fn accepts(&self, s: &str) -> bool {
		let mut current_nodes = NodeSet::new(self.nodes.len());
		let DistinguishedNodes { start, accept } = self.distinguished_nodes;
		self.add_reachable(&mut current_nodes, start);
		for c in s.chars() {
			let mut next_nodes = NodeSet::new(self.nodes.len());
			for node in current_nodes {
				for next_node in self.nodes[node].node.get_transition(c) {
					self.add_reachable(&mut next_nodes, next_node)
				}
			}
			if next_nodes.is_empty() { return false }

			current_nodes = next_nodes;
		}
		current_nodes.contains(accept)
	}
}

pub enum Regex {
	Empty,
	Dot,
	CharLiteral(char),
	StrLiteral(String),
	Concat(Vec<Regex>),
	Union(Vec<Regex>),
	OnePlus(Box<Regex>),
	Optional(Box<Regex>),
	Star(Box<Regex>),
}
impl Regex {
	fn add_fa(&self, nodes: &mut Vec<Node>) -> DistinguishedNodes {
		let mut add_node = |node| {
			let index = nodes.len();
			nodes.push(Node { node, epsilon_transitions: vec![] });
			index
		};

		use NodeType::*;
		use Regex::*;

		match self {
			Empty => Regex::Concat(vec![]).add_fa(nodes),
			Dot => {
				let accept = add_node(Terminal);
				DistinguishedNodes {
					start: add_node(All { next: accept }),
					accept,
				}
			},
			CharLiteral(c) => {
				let accept = add_node(Terminal);
				DistinguishedNodes {
					start: add_node(Exact { c: *c, next: accept }),
					accept,
				}
			},
			StrLiteral(s) => {
				let accept = add_node(Terminal);
				let mut next_node = accept;
				for c in s.chars().rev() {
					next_node = add_node(Exact { c, next: next_node })
				}
				DistinguishedNodes { start: next_node, accept }
			},
			Concat(exps) => {
				let accept = add_node(Terminal);
				let mut next_node = accept;
				for exp in exps.iter().rev() {
					let DistinguishedNodes { start, accept } = exp.add_fa(nodes);
					nodes[accept].epsilon_transitions.push(next_node);
					next_node = start;
				}
				DistinguishedNodes { start: next_node, accept }
			},
			Union(exps) => {
				let start = add_node(Terminal);
				let accept = add_node(Terminal);
				for exp in exps {
					let distinguished_nodes = exp.add_fa(nodes);
					nodes[start].epsilon_transitions.push(distinguished_nodes.start);
					nodes[distinguished_nodes.accept].epsilon_transitions.push(accept);
				}
				DistinguishedNodes { start, accept }
			},
			OnePlus(exp) => {
				let distinguished_nodes = exp.add_fa(nodes);
				let DistinguishedNodes { start, accept } = distinguished_nodes;
				nodes[accept].epsilon_transitions.push(start);
				distinguished_nodes
			},
			Optional(exp) => {
				let distinguished_nodes = exp.add_fa(nodes);
				let DistinguishedNodes { start, accept } = distinguished_nodes;
				nodes[start].epsilon_transitions.push(accept);
				distinguished_nodes
			},
			Star(exp) => {
				let distinguished_nodes = exp.add_fa(nodes);
				let DistinguishedNodes { start, accept } = distinguished_nodes;
				nodes[start].epsilon_transitions.push(accept);
				nodes[accept].epsilon_transitions.push(start);
				distinguished_nodes
			},
		}
	}
	pub fn make_fa(&self) -> NFA {
		let mut nodes = vec![];
		let distinguished_nodes = self.add_fa(&mut nodes);
		NFA { nodes, distinguished_nodes }
	}
}