#![allow(dead_code)]

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::model::{HandleType, OdbcCall, TraceHeader, TraceLog, TracedCall};

pub type SeqNum = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub seq: SeqNum,
    pub call: OdbcCall,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_line: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_line: Option<String>,
}

/// A handle's lifetime from allocation to deallocation.
///
/// Each node represents a `SQLAllocHandle` call (or a pre-existing handle discovered
/// from the trace). It contains all operations performed on the resulting handle,
/// plus child handle allocations as nested nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleNode {
    pub handle_type: HandleType,
    pub address: String,
    pub logical_name: String,
    /// The `SQLAllocHandle` call that created this handle.
    /// `None` for handles that existed before the trace started (implicit).
    pub alloc: Option<Operation>,
    /// The `SQLFreeHandle` call that destroyed this handle.
    /// `None` if the handle was never freed (e.g. truncated trace).
    pub free: Option<Operation>,
    /// Work operations performed on this handle, ordered by `seq`.
    pub operations: Vec<Operation>,
    /// Child handles allocated from this handle, ordered by alloc seq.
    pub children: Vec<HandleNode>,
}

/// The complete intermediate representation of an ODBC trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceIr {
    pub header: TraceHeader,
    pub roots: Vec<HandleNode>,
    pub unscoped_operations: Vec<Operation>,
    pub total_operations: SeqNum,
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

pub fn build_ir(trace: &TraceLog) -> TraceIr {
    let source = trace.header.source_file.as_deref();
    let mut builder = IrBuilder::new(source);
    for tc in &trace.calls {
        builder.process_call(tc.clone());
    }
    builder.finish(trace.header.clone())
}

fn make_op(seq: SeqNum, tc: TracedCall, source: Option<&str>) -> Operation {
    let fmt_line = |line: Option<usize>| -> Option<String> {
        let l = line?;
        match source {
            Some(path) => Some(format!("{path}:{l}")),
            None => Some(l.to_string()),
        }
    };
    Operation {
        seq,
        entry_line: fmt_line(tc.entry_line),
        exit_line: fmt_line(tc.exit_line),
        call: tc.call,
    }
}

struct BuilderNode {
    handle_type: HandleType,
    address: String,
    logical_name: String,
    alloc: Option<Operation>,
    free: Option<Operation>,
    operations: Vec<Operation>,
}

struct IrBuilder {
    nodes: HashMap<String, BuilderNode>,
    parent_of: HashMap<String, String>,
    /// Maps raw handle address → versioned key in `nodes`.
    active_handles: HashMap<String, String>,
    addr_generation: HashMap<String, usize>,
    unscoped: Vec<Operation>,
    seq: SeqNum,
    counters: HashMap<HandleType, usize>,
    source_file: Option<String>,
}

impl IrBuilder {
    fn new(source_file: Option<&str>) -> Self {
        Self {
            nodes: HashMap::new(),
            parent_of: HashMap::new(),
            active_handles: HashMap::new(),
            addr_generation: HashMap::new(),
            unscoped: Vec::new(),
            seq: 0,
            counters: HashMap::new(),
            source_file: source_file.map(|s| s.to_string()),
        }
    }

    fn next_seq(&mut self) -> SeqNum {
        let s = self.seq;
        self.seq += 1;
        s
    }

    fn next_logical_name(&mut self, handle_type: HandleType) -> String {
        let counter = self.counters.entry(handle_type).or_insert(0);
        let prefix = match handle_type {
            HandleType::Env => "env",
            HandleType::Dbc => "dbc",
            HandleType::Stmt => "stmt",
            HandleType::Desc => "desc",
        };
        let name = format!("{prefix}{counter}");
        *counter += 1;
        name
    }

    fn to_op(&self, seq: SeqNum, tc: TracedCall) -> Operation {
        make_op(seq, tc, self.source_file.as_deref())
    }

    fn process_call(&mut self, tc: TracedCall) {
        let seq = self.next_seq();
        match &tc.call {
            OdbcCall::AllocHandle(_) => self.process_alloc(seq, tc),
            OdbcCall::FreeHandle(_) => self.process_free(seq, tc),
            _ => self.process_regular(seq, tc),
        }
    }

    fn process_alloc(&mut self, seq: SeqNum, tc: TracedCall) {
        let (handle_type, parent_addr, child_addr, is_success) = match &tc.call {
            OdbcCall::AllocHandle(a) => (
                a.handle_type,
                a.parent_handle.clone(),
                a.child_handle.clone(),
                a.return_code.is_success(),
            ),
            _ => unreachable!(),
        };

        if !is_success {
            self.unscoped.push(self.to_op(seq, tc));
            return;
        }

        match (handle_type, child_addr) {
            (Some(ht), Some(raw_child)) => {
                if self.active_handles.contains_key(&raw_child) {
                    self.active_handles.remove(&raw_child);
                }

                let gen = self.addr_generation.entry(raw_child.clone()).or_insert(0);
                let vkey = versioned_key(&raw_child, *gen);
                if self.nodes.contains_key(&vkey) {
                    *gen += 1;
                }
                let vkey = versioned_key(&raw_child, *gen);

                let logical_name = self.next_logical_name(ht);
                self.nodes.insert(
                    vkey.clone(),
                    BuilderNode {
                        handle_type: ht,
                        address: raw_child.clone(),
                        logical_name,
                        alloc: Some(self.to_op(seq, tc)),
                        free: None,
                        operations: Vec::new(),
                    },
                );
                self.active_handles.insert(raw_child.clone(), vkey.clone());

                if let Some(raw_parent) = parent_addr {
                    if !is_null_handle(&raw_parent) {
                        let parent_vkey = self
                            .active_handles
                            .get(&raw_parent)
                            .cloned()
                            .unwrap_or(raw_parent);
                        self.parent_of.insert(vkey, parent_vkey);
                    }
                }
            }
            _ => {
                self.unscoped.push(self.to_op(seq, tc));
            }
        }
    }

    fn process_free(&mut self, seq: SeqNum, tc: TracedCall) {
        let source = self.source_file.as_deref().map(|s| s.to_string());
        let raw_addr = tc.call.handle_addr().map(|s| s.to_string());
        if let Some(raw_addr) = raw_addr {
            if let Some(vkey) = self.active_handles.remove(&raw_addr) {
                if let Some(node) = self.nodes.get_mut(&vkey) {
                    node.free = Some(make_op(seq, tc, source.as_deref()));
                    return;
                }
            }
        }
        self.unscoped.push(make_op(seq, tc, source.as_deref()));
    }

    fn process_regular(&mut self, seq: SeqNum, tc: TracedCall) {
        let source = self.source_file.as_deref().map(|s| s.to_string());
        if let Some(vkey) = find_owning_handle(&tc.call, &self.active_handles) {
            self.nodes
                .get_mut(&vkey)
                .expect("active handle must have a node")
                .operations
                .push(make_op(seq, tc, source.as_deref()));
            return;
        }

        if let Some((raw_addr, ht)) = find_implicit_handle(&tc.call) {
            let logical_name = self.next_logical_name(ht);
            let vkey = raw_addr.clone();
            self.nodes.insert(
                vkey.clone(),
                BuilderNode {
                    handle_type: ht,
                    address: raw_addr.clone(),
                    logical_name,
                    alloc: None,
                    free: None,
                    operations: vec![make_op(seq, tc, source.as_deref())],
                },
            );
            self.active_handles.insert(raw_addr, vkey);
            return;
        }

        self.unscoped.push(make_op(seq, tc, source.as_deref()));
    }

    fn finish(mut self, header: TraceHeader) -> TraceIr {
        let total = self.seq;
        self.infer_implicit_parents();

        let mut children_of: HashMap<String, Vec<String>> = HashMap::new();
        let mut root_addrs: Vec<String> = Vec::new();

        let all_addrs: Vec<String> = self.nodes.keys().cloned().collect();
        for addr in &all_addrs {
            match self.parent_of.get(addr) {
                Some(parent) if self.nodes.contains_key(parent) => {
                    children_of
                        .entry(parent.clone())
                        .or_default()
                        .push(addr.clone());
                }
                _ => {
                    root_addrs.push(addr.clone());
                }
            }
        }

        root_addrs.sort_by_key(|a| {
            let node = &self.nodes[a];
            node.alloc
                .as_ref()
                .map(|op| op.seq)
                .or_else(|| node.operations.first().map(|op| op.seq))
                .unwrap_or(SeqNum::MAX)
        });

        let roots: Vec<HandleNode> = root_addrs
            .iter()
            .filter_map(|a| build_subtree(a, &mut self.nodes, &children_of))
            .collect();

        let mut ir = TraceIr {
            header,
            roots,
            unscoped_operations: self.unscoped,
            total_operations: total,
        };
        ir.resolve_all_handles();
        ir
    }

    /// When there is exactly one Env, parent any orphan Dbc nodes to it.
    /// When there is exactly one Dbc, parent any orphan Stmt nodes to it.
    fn infer_implicit_parents(&mut self) {
        let env_addrs: Vec<String> = self
            .nodes
            .iter()
            .filter(|(_, n)| n.handle_type == HandleType::Env)
            .map(|(a, _)| a.clone())
            .collect();

        if env_addrs.len() == 1 {
            let orphan_dbcs: Vec<String> = self
                .nodes
                .iter()
                .filter(|(addr, n)| {
                    n.handle_type == HandleType::Dbc && !self.parent_of.contains_key(*addr)
                })
                .map(|(a, _)| a.clone())
                .collect();
            for dbc in orphan_dbcs {
                self.parent_of.insert(dbc, env_addrs[0].clone());
            }
        }

        let dbc_addrs: Vec<String> = self
            .nodes
            .iter()
            .filter(|(_, n)| n.handle_type == HandleType::Dbc)
            .map(|(a, _)| a.clone())
            .collect();

        if dbc_addrs.len() == 1 {
            let orphan_stmts: Vec<String> = self
                .nodes
                .iter()
                .filter(|(addr, n)| {
                    n.handle_type == HandleType::Stmt && !self.parent_of.contains_key(*addr)
                })
                .map(|(a, _)| a.clone())
                .collect();
            for stmt in orphan_stmts {
                self.parent_of.insert(stmt, dbc_addrs[0].clone());
            }
        }
    }
}

fn build_subtree(
    addr: &str,
    nodes: &mut HashMap<String, BuilderNode>,
    children_of: &HashMap<String, Vec<String>>,
) -> Option<HandleNode> {
    let node = nodes.remove(addr)?;
    let child_addrs = children_of.get(addr).cloned().unwrap_or_default();
    let mut children: Vec<HandleNode> = child_addrs
        .iter()
        .filter_map(|a| build_subtree(a, nodes, children_of))
        .collect();
    children.sort_by_key(|c| {
        c.alloc
            .as_ref()
            .map(|op| op.seq)
            .or_else(|| c.operations.first().map(|op| op.seq))
            .unwrap_or(SeqNum::MAX)
    });

    Some(HandleNode {
        handle_type: node.handle_type,
        address: node.address,
        logical_name: node.logical_name,
        alloc: node.alloc,
        free: node.free,
        operations: node.operations,
        children,
    })
}

// ---------------------------------------------------------------------------
// Handle attribution helpers
// ---------------------------------------------------------------------------

/// Returns the versioned key of the node that owns this call, using the
/// typed `handle_addr()` accessor on `OdbcCall`.
fn find_owning_handle(call: &OdbcCall, active: &HashMap<String, String>) -> Option<String> {
    call.handle_addr()
        .and_then(|addr| active.get(addr).cloned())
}

/// For a call whose handle is not yet tracked, infer the handle type from
/// the enum variant and return the address + type for implicit node creation.
fn find_implicit_handle(call: &OdbcCall) -> Option<(String, HandleType)> {
    let addr = call.handle_addr()?;
    let ht = match call {
        OdbcCall::SetEnvAttr(_) => HandleType::Env,
        OdbcCall::DriverConnect(_)
        | OdbcCall::Disconnect(_)
        | OdbcCall::SetConnectAttr(_)
        | OdbcCall::GetInfo(_)
        | OdbcCall::GetFunctions(_) => HandleType::Dbc,
        OdbcCall::Prepare(_)
        | OdbcCall::Execute(_)
        | OdbcCall::ExecDirect(_)
        | OdbcCall::NumResultCols(_)
        | OdbcCall::DescribeCol(_)
        | OdbcCall::Fetch(_)
        | OdbcCall::FetchScroll(_)
        | OdbcCall::GetData(_)
        | OdbcCall::RowCount(_)
        | OdbcCall::MoreResults(_)
        | OdbcCall::CloseCursor(_) => HandleType::Stmt,
        OdbcCall::GetDiagRec(c) => c.handle_type?,
        _ => return None,
    };
    Some((addr.to_string(), ht))
}

fn versioned_key(addr: &str, gen: usize) -> String {
    if gen == 0 {
        addr.to_string()
    } else {
        format!("{addr}#{gen}")
    }
}

fn is_null_handle(addr: &str) -> bool {
    matches!(
        addr,
        "SQL_NULL_HANDLE" | "(nil)" | "0x0" | "0x00" | "0" | "NULL"
    )
}

// ---------------------------------------------------------------------------
// Traversal helpers
// ---------------------------------------------------------------------------

impl TraceIr {
    /// Flatten all operations (including alloc/free) across the entire tree,
    /// sorted by global sequence number.
    pub fn all_operations_sorted(&self) -> Vec<&Operation> {
        let mut ops: Vec<&Operation> = Vec::new();
        for op in &self.unscoped_operations {
            ops.push(op);
        }
        for root in &self.roots {
            root.collect_all_operations(&mut ops);
        }
        ops.sort_by_key(|op| op.seq);
        ops
    }

    pub fn handle_count(&self) -> usize {
        self.roots.iter().map(|r| r.subtree_handle_count()).sum()
    }

    pub fn all_handles(&self) -> Vec<&HandleNode> {
        let mut handles = Vec::new();
        for root in &self.roots {
            root.collect_handles(&mut handles);
        }
        handles
    }

    /// Build an address→logical_name map from all handles and replace raw
    /// addresses with logical names in every operation's call fields.
    fn resolve_all_handles(&mut self) {
        let map: HashMap<String, String> = self
            .all_handles()
            .into_iter()
            .map(|h| (h.address.clone(), h.logical_name.clone()))
            .collect();
        for root in &mut self.roots {
            root.resolve_handles(&map);
        }
        for op in &mut self.unscoped_operations {
            op.call.resolve_handles(&map);
        }
    }

    /// Flatten all calls in sequence order (for consumers that need a linear list).
    pub fn flatten_calls(&self) -> Vec<OdbcCall> {
        self.all_operations_sorted()
            .into_iter()
            .map(|op| op.call.clone())
            .collect()
    }

    /// Split the IR by handle type. Each returned pair is (logical_name, sub-IR).
    pub fn split(&self, mode: SplitMode) -> Vec<(String, TraceIr)> {
        match mode {
            SplitMode::Env => self
                .roots
                .iter()
                .map(|env| {
                    let name = env.logical_name.clone();
                    let total = env.all_operations_recursive().len() as SeqNum;
                    (
                        name,
                        TraceIr {
                            header: self.header.clone(),
                            roots: vec![env.clone()],
                            unscoped_operations: vec![],
                            total_operations: total,
                        },
                    )
                })
                .collect(),
            SplitMode::Connection => {
                let mut result = Vec::new();
                for env in &self.roots {
                    for dbc in &env.children {
                        if dbc.handle_type != HandleType::Dbc {
                            continue;
                        }
                        let wrapped = wrap_in_parent(env, dbc.clone());
                        let total = wrapped.all_operations_recursive().len() as SeqNum;
                        result.push((
                            dbc.logical_name.clone(),
                            TraceIr {
                                header: self.header.clone(),
                                roots: vec![wrapped],
                                unscoped_operations: vec![],
                                total_operations: total,
                            },
                        ));
                    }
                }
                result
            }
            SplitMode::Statement => {
                let mut result = Vec::new();
                for env in &self.roots {
                    for dbc in &env.children {
                        if dbc.handle_type != HandleType::Dbc {
                            continue;
                        }
                        for stmt in &dbc.children {
                            if stmt.handle_type != HandleType::Stmt {
                                continue;
                            }
                            let dbc_wrapped = wrap_in_parent(dbc, stmt.clone());
                            let env_wrapped = wrap_in_parent(env, dbc_wrapped);
                            let total = env_wrapped.all_operations_recursive().len() as SeqNum;
                            result.push((
                                stmt.logical_name.clone(),
                                TraceIr {
                                    header: self.header.clone(),
                                    roots: vec![env_wrapped],
                                    unscoped_operations: vec![],
                                    total_operations: total,
                                },
                            ));
                        }
                    }
                }
                result
            }
        }
    }
}

/// Clone a parent node's metadata and operations, replacing its children
/// with a single child node.
fn wrap_in_parent(parent: &HandleNode, child: HandleNode) -> HandleNode {
    HandleNode {
        handle_type: parent.handle_type,
        address: parent.address.clone(),
        logical_name: parent.logical_name.clone(),
        alloc: parent.alloc.clone(),
        free: parent.free.clone(),
        operations: parent.operations.clone(),
        children: vec![child],
    }
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum SplitMode {
    Env,
    Connection,
    Statement,
}

pub fn load_ir_yaml(path: &std::path::Path) -> std::io::Result<TraceIr> {
    let content = std::fs::read_to_string(path)?;
    serde_yaml::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{e}")))
}

impl HandleNode {
    pub fn is_implicit(&self) -> bool {
        self.alloc.is_none()
    }

    /// Whether any SQL query in this subtree was truncated by the trace.
    pub fn has_truncated_sql(&self) -> bool {
        self.operations.iter().any(|op| op.call.has_truncated_sql())
            || self.children.iter().any(|c| c.has_truncated_sql())
    }

    /// Replace raw handle addresses with logical names in all operations.
    fn resolve_handles(&mut self, map: &HashMap<String, String>) {
        if let Some(alloc) = &mut self.alloc {
            alloc.call.resolve_handles(map);
        }
        if let Some(free) = &mut self.free {
            free.call.resolve_handles(map);
        }
        for op in &mut self.operations {
            op.call.resolve_handles(map);
        }
        for child in &mut self.children {
            child.resolve_handles(map);
        }
    }

    /// All operations in this subtree (including alloc, free, and children),
    /// sorted by global sequence number.
    pub fn all_operations_recursive(&self) -> Vec<&Operation> {
        let mut ops = Vec::new();
        self.collect_all_operations(&mut ops);
        ops.sort_by_key(|op| op.seq);
        ops
    }

    pub fn handles_by_type(&self, ht: HandleType) -> Vec<&HandleNode> {
        let mut result = Vec::new();
        self.collect_handles_by_type(ht, &mut result);
        result
    }

    fn collect_all_operations<'a>(&'a self, ops: &mut Vec<&'a Operation>) {
        if let Some(alloc) = &self.alloc {
            ops.push(alloc);
        }
        for op in &self.operations {
            ops.push(op);
        }
        if let Some(free) = &self.free {
            ops.push(free);
        }
        for child in &self.children {
            child.collect_all_operations(ops);
        }
    }

    fn collect_handles_by_type<'a>(&'a self, ht: HandleType, out: &mut Vec<&'a HandleNode>) {
        if self.handle_type == ht {
            out.push(self);
        }
        for child in &self.children {
            child.collect_handles_by_type(ht, out);
        }
    }

    fn collect_handles<'a>(&'a self, out: &mut Vec<&'a HandleNode>) {
        out.push(self);
        for child in &self.children {
            child.collect_handles(out);
        }
    }

    fn subtree_handle_count(&self) -> usize {
        1 + self
            .children
            .iter()
            .map(|c| c.subtree_handle_count())
            .sum::<usize>()
    }
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

fn handle_type_label(ht: HandleType) -> &'static str {
    match ht {
        HandleType::Env => "Env",
        HandleType::Dbc => "Dbc",
        HandleType::Stmt => "Stmt",
        HandleType::Desc => "Desc",
    }
}

impl fmt::Display for TraceIr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "TraceIr: {} operations, {} handles",
            self.total_operations,
            self.handle_count()
        )?;

        if !self.unscoped_operations.is_empty() {
            writeln!(f, "  unscoped ({}):", self.unscoped_operations.len())?;
            for op in &self.unscoped_operations {
                writeln!(f, "    [{:>4}] {}", op.seq, op.call)?;
            }
        }

        for root in &self.roots {
            write_node(f, root, 1)?;
        }

        Ok(())
    }
}

fn write_node(f: &mut fmt::Formatter<'_>, node: &HandleNode, depth: usize) -> fmt::Result {
    let indent = "  ".repeat(depth);
    let inner = "  ".repeat(depth + 1);

    if let Some(alloc) = &node.alloc {
        writeln!(
            f,
            "{}{} ({}, {}) [alloc: {}]",
            indent,
            node.logical_name,
            handle_type_label(node.handle_type),
            node.address,
            alloc.seq
        )?;
    } else {
        writeln!(
            f,
            "{}{} ({}, {}) [implicit]",
            indent,
            node.logical_name,
            handle_type_label(node.handle_type),
            node.address
        )?;
    }

    for op in &node.operations {
        writeln!(f, "{}[{:>4}] {}", inner, op.seq, op.call)?;
    }

    if let Some(free) = &node.free {
        writeln!(f, "{}free: [{}] {}", inner, free.seq, free.call)?;
    }

    for child in &node.children {
        write_node(f, child, depth + 1)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::iodbc;

    const SAMPLE_TRACE: &str = include_str!("../../odbc_tests/tests/replay/iodbctest/select_1.log");

    #[test]
    fn test_build_ir_basic_structure() {
        let trace = iodbc::parse_str(SAMPLE_TRACE).expect("parse failed");
        let ir = build_ir(&trace);

        assert_eq!(ir.roots.len(), 1, "single env root");

        let env = &ir.roots[0];
        assert_eq!(env.handle_type, HandleType::Env);
        assert!(
            env.is_implicit(),
            "env was not explicitly allocated in trace"
        );
        assert!(env.free.is_some(), "env was freed");

        assert_eq!(env.children.len(), 1, "one dbc under env");
        let dbc = &env.children[0];
        assert_eq!(dbc.handle_type, HandleType::Dbc);
        assert!(
            dbc.is_implicit(),
            "dbc was not explicitly allocated in trace"
        );
        assert!(dbc.free.is_some(), "dbc was freed");

        assert_eq!(dbc.children.len(), 1, "one stmt under dbc");
        let stmt = &dbc.children[0];
        assert_eq!(stmt.handle_type, HandleType::Stmt);
        assert!(!stmt.is_implicit(), "stmt was explicitly allocated");
        assert!(stmt.free.is_some(), "stmt was freed");
    }

    #[test]
    fn test_handle_count() {
        let trace = iodbc::parse_str(SAMPLE_TRACE).expect("parse failed");
        let ir = build_ir(&trace);
        assert_eq!(ir.handle_count(), 3);
    }

    #[test]
    fn test_operations_attributed_correctly() {
        let trace = iodbc::parse_str(SAMPLE_TRACE).expect("parse failed");
        let ir = build_ir(&trace);

        let env = &ir.roots[0];
        let dbc = &env.children[0];
        let stmt = &dbc.children[0];

        let env_fn_names: Vec<&str> = env
            .operations
            .iter()
            .map(|o| o.call.function_name())
            .collect();
        assert!(
            env_fn_names.contains(&"SQLGetDiagRec"),
            "env should have GetDiagRec, got {env_fn_names:?}"
        );

        let dbc_fn_names: Vec<&str> = dbc
            .operations
            .iter()
            .map(|o| o.call.function_name())
            .collect();
        assert!(dbc_fn_names.contains(&"SQLDriverConnect"));
        assert!(dbc_fn_names.contains(&"SQLGetInfo"));
        assert!(dbc_fn_names.contains(&"SQLDisconnect"));

        let stmt_fn_names: Vec<&str> = stmt
            .operations
            .iter()
            .map(|o| o.call.function_name())
            .collect();
        assert!(stmt_fn_names.contains(&"SQLPrepare"));
        assert!(stmt_fn_names.contains(&"SQLExecute"));
        assert!(stmt_fn_names.contains(&"SQLFetchScroll"));
        assert!(stmt_fn_names.contains(&"SQLGetData"));
    }

    #[test]
    fn test_global_sequence_is_monotonic() {
        let trace = iodbc::parse_str(SAMPLE_TRACE).expect("parse failed");
        let ir = build_ir(&trace);
        let all_ops = ir.all_operations_sorted();

        for window in all_ops.windows(2) {
            assert!(
                window[0].seq < window[1].seq,
                "sequence must be strictly increasing: {} >= {}",
                window[0].seq,
                window[1].seq
            );
        }
    }

    #[test]
    fn test_all_operations_accounted_for() {
        let trace = iodbc::parse_str(SAMPLE_TRACE).expect("parse failed");
        let num_calls = trace.calls.len() as SeqNum;
        let ir = build_ir(&trace);

        assert_eq!(ir.total_operations, num_calls);

        let all_ops = ir.all_operations_sorted();
        assert_eq!(
            all_ops.len(),
            num_calls as usize,
            "every call must appear somewhere in the IR"
        );
    }

    #[test]
    fn test_no_unscoped_operations() {
        let trace = iodbc::parse_str(SAMPLE_TRACE).expect("parse failed");
        let ir = build_ir(&trace);

        assert!(
            ir.unscoped_operations.is_empty(),
            "all operations should be attributed, got {} unscoped: {:?}",
            ir.unscoped_operations.len(),
            ir.unscoped_operations
                .iter()
                .map(|o| o.call.function_name())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_handles_by_type() {
        let trace = iodbc::parse_str(SAMPLE_TRACE).expect("parse failed");
        let ir = build_ir(&trace);

        let all_handles = ir.all_handles();
        let envs: Vec<_> = all_handles
            .iter()
            .filter(|h| h.handle_type == HandleType::Env)
            .collect();
        let stmts: Vec<_> = all_handles
            .iter()
            .filter(|h| h.handle_type == HandleType::Stmt)
            .collect();

        assert_eq!(envs.len(), 1);
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn test_display_does_not_panic() {
        let trace = iodbc::parse_str(SAMPLE_TRACE).expect("parse failed");
        let ir = build_ir(&trace);
        let output = format!("{ir}");
        assert!(output.contains("TraceIr:"));
        assert!(output.contains("[implicit]"));
        assert!(output.contains("SQLPrepare"));
    }

    // -- unixODBC format tests --

    const UNIXODBC_TRACE: &str = "\
[ODBC][118][1774615098.017111][__handles.c][499]
\t\tExit:[SQL_SUCCESS]
\t\t\tEnvironment = 0x2e91620
[ODBC][118][1774615098.017167][SQLSetEnvAttr.c][189]
\t\tEntry:
\t\t\tEnvironment = 0x2e91620
\t\t\tAttribute = SQL_ATTR_ODBC_VERSION
\t\t\tValue = 0x17c
\t\t\tStrLen = 0
[ODBC][118][1774615098.017193][SQLSetEnvAttr.c][381]
\t\tExit:[SQL_SUCCESS]
[ODBC][118][1774615098.017216][SQLAllocHandle.c][395]
\t\tEntry:
\t\t\tHandle Type = 2
\t\t\tInput Handle = 0x2e91620
\t\tUNICODE Using encoding ASCII 'UTF-8' and UNICODE 'UTF16LE'

[ODBC][118][1774615098.017363][SQLAllocHandle.c][531]
\t\tExit:[SQL_SUCCESS]
\t\t\tOutput Handle = 0x2e92330
[ODBC][118][1774615098.017499][SQLDriverConnect.c][751]
\t\tEntry:
\t\t\tConnection = 0x2e92330
\t\t\tWindow Hdl = (nil)
\t\t\tStr In = [Driver=TestDriver;SERVER=test.snowflakecomputing.com][length = 52]
\t\t\tStr Out = 0x7ffc81545ea0
\t\t\tStr Out Max = 1024
\t\t\tStr Out Ptr = 0x7ffc81545e8a
\t\t\tCompletion = 0
[ODBC][118][1774615098.123288][SQLDriverConnect.c][1809]
\t\tExit:[SQL_SUCCESS]
[ODBC][118][1774615098.163786][SQLAllocHandle.c][578]
\t\tEntry:
\t\t\tHandle Type = 3
\t\t\tInput Handle = 0x2e92330
[ODBC][118][1774615098.163973][SQLAllocHandle.c][1123]
\t\tExit:[SQL_SUCCESS]
\t\t\tOutput Handle = 0x3086810
[ODBC][118][1774615098.164003][SQLExecDirect.c][240]
\t\tEntry:
\t\t\tStatement = 0x3086810
\t\t\tSQL = [SELECT 1][length = 8 (SQL_NTS)]
[ODBC][118][1774615098.237948][SQLExecDirect.c][521]
\t\tExit:[SQL_SUCCESS]
[ODBC][118][1774615098.238178][SQLFetch.c][162]
\t\tEntry:
\t\t\tStatement = 0x3086810
[ODBC][118][1774615098.238253][SQLFetch.c][352]
\t\tExit:[SQL_NO_DATA]
[ODBC][118][1774615098.238366][SQLFreeHandle.c][387]
\t\tEntry:
\t\t\tHandle Type = 3
\t\t\tInput Handle = 0x3086810
[ODBC][118][1774615098.238473][SQLFreeHandle.c][490]
\t\tExit:[SQL_SUCCESS]
[ODBC][118][1774615098.509432][SQLDisconnect.c][208]
\t\tEntry:
\t\t\tConnection = 0x2e92330
[ODBC][118][1774615098.511068][SQLDisconnect.c][358]
\t\tExit:[SQL_SUCCESS]
[ODBC][118][1774615098.511124][SQLFreeHandle.c][290]
\t\tEntry:
\t\t\tHandle Type = 2
\t\t\tInput Handle = 0x2e92330
[ODBC][118][1774615098.511160][SQLFreeHandle.c][339]
\t\tExit:[SQL_SUCCESS]
[ODBC][118][1774615098.511189][SQLFreeHandle.c][220]
\t\tEntry:
\t\t\tHandle Type = 1
\t\t\tInput Handle = 0x2e91620
[ODBC][118][1774615098.511200][SQLFreeHandle.c][250]
\t\tExit:[SQL_SUCCESS]
";

    use crate::parser::unixodbc;

    #[test]
    fn test_unixodbc_tree_structure() {
        let trace = unixodbc::parse_str(UNIXODBC_TRACE).expect("parse failed");
        let ir = build_ir(&trace);

        assert_eq!(ir.roots.len(), 1, "single env root");
        assert_eq!(ir.handle_count(), 3);
        assert!(ir.unscoped_operations.is_empty(), "no unscoped ops");

        let env = &ir.roots[0];
        assert_eq!(env.handle_type, HandleType::Env);
        assert!(!env.is_implicit(), "env explicitly allocated via __handles");
        assert!(env.free.is_some());
        assert_eq!(env.operations.len(), 1, "SetEnvAttr");

        let dbc = &env.children[0];
        assert_eq!(dbc.handle_type, HandleType::Dbc);
        assert!(!dbc.is_implicit());
        assert!(dbc.free.is_some());

        let dbc_fns: Vec<&str> = dbc
            .operations
            .iter()
            .map(|o| o.call.function_name())
            .collect();
        assert!(dbc_fns.contains(&"SQLDriverConnect"));
        assert!(dbc_fns.contains(&"SQLDisconnect"));

        let stmt = &dbc.children[0];
        assert_eq!(stmt.handle_type, HandleType::Stmt);
        assert!(!stmt.is_implicit());
        assert!(stmt.free.is_some());

        let stmt_fns: Vec<&str> = stmt
            .operations
            .iter()
            .map(|o| o.call.function_name())
            .collect();
        assert!(stmt_fns.contains(&"SQLExecDirect"));
        assert!(stmt_fns.contains(&"SQLFetch"));
    }

    #[test]
    fn test_unixodbc_all_operations_accounted_for() {
        let trace = unixodbc::parse_str(UNIXODBC_TRACE).expect("parse failed");
        let num_calls = trace.calls.len() as SeqNum;
        let ir = build_ir(&trace);

        assert_eq!(ir.total_operations, num_calls);
        assert_eq!(ir.all_operations_sorted().len(), num_calls as usize);
    }

    #[test]
    fn test_unixodbc_global_sequence_monotonic() {
        let trace = unixodbc::parse_str(UNIXODBC_TRACE).expect("parse failed");
        let ir = build_ir(&trace);
        let all_ops = ir.all_operations_sorted();
        for window in all_ops.windows(2) {
            assert!(window[0].seq < window[1].seq);
        }
    }
}
