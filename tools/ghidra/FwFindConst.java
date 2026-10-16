// Finds instructions with specific immediates and reports the containing function.
// -postScript FwFindConst.java 0x90 0x107 0x108
//@category KM780
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.*;
import ghidra.program.model.scalar.Scalar;
import ghidra.program.model.address.Address;
import java.util.*;

public class FwFindConst extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        Set<Long> want = new TreeSet<>();
        for (String a : args) want.add(Long.parseLong(a.replace("0x", ""), 16));
        FunctionManager fm = currentProgram.getFunctionManager();
        Map<Long, List<String>> byval = new TreeMap<>();
        InstructionIterator it = currentProgram.getListing().getInstructions(true);
        while (it.hasNext()) {
            Instruction ins = it.next();
            for (int op = 0; op < ins.getNumOperands(); op++) {
                Scalar sc = ins.getScalar(op);
                if (sc == null) continue;
                long v = sc.getValue();
                if (!want.contains(v)) continue;
                Function f = fm.getFunctionContaining(ins.getAddress());
                byval.computeIfAbsent(v, k -> new ArrayList<>()).add(
                    String.format("%s in %s  [%s]", ins.getAddress(),
                        f == null ? "(outside a function)" : f.getEntryPoint().toString(),
                        ins.toString()));
            }
        }
        for (Long v : want) {
            List<String> l = byval.getOrDefault(v, List.of());
            println(String.format("### immediate 0x%x -> %d occurrence(s)", v, l.size()));
            for (String s : l) println("   " + s);
        }
        // also: data regions holding the word 0x107/0x108 referenced by code
        println("### referenced data regions holding the value 144/263/264:");
        for (Long v : want) {
            Address a = currentProgram.getMinAddress();
            // simple halfword sweep
            byte[] buf = new byte[(int) currentProgram.getMaxAddress()
                .subtract(currentProgram.getMinAddress()) + 1];
            currentProgram.getMemory().getBytes(a, buf);
            for (int i = 0; i + 1 < buf.length; i += 2) {
                int hw = (buf[i] & 0xff) | ((buf[i + 1] & 0xff) << 8);
                if (hw == v.intValue()) {
                    Address at = a.add(i);
                    int n = 0;
                    StringBuilder sb = new StringBuilder();
                    for (var r : getReferencesTo(at)) { n++; sb.append(r.getFromAddress()).append(" "); }
                    if (n > 0) println(String.format("   0x%x at %s : xrefs %s", v, at, sb));
                }
            }
        }
    }
}
