// Decompiles functions from the Ghidra project by address.
// analyzeHeadless <proj> KM780 -process <bin> -noanalysis \
//   -scriptPath tools/ghidra -postScript FwDecomp.java 0x3ffe [0x...]
//@category KM780
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;

public class FwDecomp extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length == 0) { println("usage: FwDecomp.java <addr> [addr...]"); return; }
        DecompInterface di = new DecompInterface();
        di.openProgram(currentProgram);
        di.setSimplificationStyle("decompile");
        for (String a : args) {
            Address addr = toAddr(Long.parseLong(a.replace("0x", ""), 16));
            Function f = getFunctionContaining(addr);
            if (f == null) { println("=== " + a + " : no function"); continue; }
            println("=== " + f.getEntryPoint() + "  (" + f.getBody().getNumAddresses() + " bytes)");
            DecompileResults r = di.decompileFunction(f, 120, monitor);
            if (r.decompileCompleted()) println(r.getDecompiledFunction().getC());
            else println("  FAILED: " + r.getErrorMessage());
        }
        di.dispose();
    }
}
