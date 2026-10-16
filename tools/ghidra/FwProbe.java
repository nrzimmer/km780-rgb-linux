// Inventory and dispatcher hunt in the KM780 firmware.
// analyzeHeadless <proj> KM780 -process fw77.bin -noanalysis \
//   -scriptPath tools/ghidra -postScript FwProbe.java
//@category KM780
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.*;
import ghidra.program.model.scalar.Scalar;
import ghidra.program.model.symbol.Reference;
import java.util.*;

public class FwProbe extends GhidraScript {

    @Override
    public void run() throws Exception {
        FunctionManager fm = currentProgram.getFunctionManager();
        Listing listing = currentProgram.getListing();

        List<Function> funcs = new ArrayList<>();
        for (Function f : fm.getFunctions(true)) funcs.add(f);
        println("### " + funcs.size() + " functions recognised");

        funcs.sort((a, b) -> Long.compare(b.getBody().getNumAddresses(),
                                          a.getBody().getNumAddresses()));
        println("### 12 largest functions:");
        for (int i = 0; i < Math.min(12, funcs.size()); i++) {
            Function f = funcs.get(i);
            println(String.format("   %-12s %5d bytes",
                f.getEntryPoint(), f.getBody().getNumAddresses()));
        }

        println("### xrefs to addresses of interest:");
        long[] addrs = {0x979cL, 0xa432L, 0xa37aL, 0xa2d5L, 0xa444L};
        String[] names = {"descriptor ptr table", "device desc",
                          "rptdesc ID7", "rptdesc mouse", "config desc"};
        for (int i = 0; i < addrs.length; i++) {
            Address a = toAddr(addrs[i]);
            StringBuilder sb = new StringBuilder();
            int n = 0;
            for (Reference r : getReferencesTo(a)) {
                if (n++ < 6) sb.append(r.getFromAddress()).append(" ");
            }
            println(String.format("   %-24s 0x%04x : %d xref(s)  %s",
                names[i], addrs[i], n, sb));
        }

        // functions with the most comparisons to distinct immediates = dispatcher candidates
        Map<Address, Set<Long>> cmps = new HashMap<>();
        InstructionIterator it = listing.getInstructions(true);
        while (it.hasNext()) {
            Instruction ins = it.next();
            String mn = ins.getMnemonicString().toLowerCase();
            if (!(mn.startsWith("cmp") || mn.startsWith("sub") || mn.startsWith("cmn")))
                continue;
            for (int op = 0; op < ins.getNumOperands(); op++) {
                Scalar sc = ins.getScalar(op);
                if (sc == null) continue;
                long v = sc.getValue();
                if (v < 0 || v > 0xff) continue;
                Function f = fm.getFunctionContaining(ins.getAddress());
                if (f == null) continue;
                cmps.computeIfAbsent(f.getEntryPoint(), k -> new TreeSet<>()).add(v);
            }
        }
        List<Map.Entry<Address, Set<Long>>> rank = new ArrayList<>(cmps.entrySet());
        rank.sort((a, b) -> Integer.compare(b.getValue().size(), a.getValue().size()));
        println("### functions with the most distinct immediates in cmp/sub (dispatcher candidates):");
        for (int i = 0; i < Math.min(12, rank.size()); i++) {
            Set<Long> s = rank.get(i).getValue();
            StringBuilder sb = new StringBuilder();
            int k = 0;
            for (Long v : s) { if (k++ < 20) sb.append(String.format("0x%02x ", v)); }
            println(String.format("   %-12s %2d distinct: %s",
                rank.get(i).getKey(), s.size(), sb));
        }

        // functions that compare against SEVERAL of our command bytes
        long[] CMD = {0x01, 0x02, 0x05, 0x09, 0x0a, 0x0d, 0x12, 0x16};
        println("### functions comparing against >=3 known command bytes:");
        for (Map.Entry<Address, Set<Long>> e : cmps.entrySet()) {
            List<String> hit = new ArrayList<>();
            for (long c : CMD) if (e.getValue().contains(c)) hit.add(String.format("0x%02x", c));
            if (hit.size() >= 3)
                println(String.format("   %-12s %d/%d: %s",
                    e.getKey(), hit.size(), CMD.length, String.join(" ", hit)));
        }
    }
}
