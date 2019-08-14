use std::collections::HashSet;

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

pub struct NFA {
	nodes: Vec<Node>,
	distinguished_nodes: DistinguishedNodes,
}
impl NFA {
	fn add_reachable(&self, nodes: &mut HashSet<NodeIndex>, start: NodeIndex) {
		if nodes.insert(start) {
			for node in &self.nodes[start].epsilon_transitions {
				self.add_reachable(nodes, *node)
			}
		}
	}
	pub fn accepts(&self, s: &str) -> bool {
		let mut current_nodes = HashSet::new();
		let DistinguishedNodes { start, accept } = self.distinguished_nodes;
		self.add_reachable(&mut current_nodes, start);
		for c in s.chars() {
			let mut next_nodes = HashSet::new();
			for node in current_nodes {
				if let Some(next_node) = self.nodes[node].node.get_transition(c) {
					self.add_reachable(&mut next_nodes, next_node)
				}
			}
			if next_nodes.is_empty() { return false }

			current_nodes = next_nodes;
		}
		current_nodes.contains(&accept)
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