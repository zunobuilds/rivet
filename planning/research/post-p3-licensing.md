# GPL-family licensing decision research

**Owner adoption update:** The owner subsequently instructed applying the recommended GPL-3.0-only license to this repository. [LICENSE](../../LICENSE), README and Cargo metadata now record that choice for Rivet's original code, documentation and examples, except where otherwise noted. Third-party terms remain intact. Official-mod license choices for other repositories are not made by this change. The comparison and pending-adoption language below are preserved as historical research; distribution/compliance questions remain applicable.


No license is selected. This comparison is a planning aid for owner/legal review, not license adoption or a legal clearance. No LICENSE/SPDX code headers/package license field were added. The owner's stated goal is that covered distributed derivatives remain open-source; ordinary public visibility does not supply that permission.

## Options

| Option | Reciprocity and practical fit |
| --- | --- |
| GPL-3.0-only | Strong copyleft when covered works are conveyed; recipients receive required source/rights under v3. No automatic adoption of later license versions. Best match for the stated distribution-focused goal. |
| GPL-3.0-or-later | Same v3 baseline, recipients may choose an FSF later version. More future compatibility flexibility, less owner control over future applicable terms. |
| AGPL-3.0-only / -or-later | GPL-family distribution obligations plus section 13 source offer for users interacting remotely with a modified covered program. Consider if hosted modified Rivet services are an explicit reciprocity goal. Not a blanket rule that every web/API caller's code becomes open-source. |
| LGPL-3.0-only / -or-later | Copyleft for library changes while allowing qualifying combined applications under other terms, with relinking/reverse-engineering and notice/source conditions. Better for a deliberately reusable library than strong copyleft over the whole platform. |
| GPL-2.0-only / -or-later | Relevant to Linux/Git/tools; v2-only lacks v3's compatibility with Apache-2.0 code. “Or later” can allow a v3 route. Separate programs/components need artifact-specific review. |
| MPL-2.0 | File-level reciprocity with broader combined-work flexibility; less protection over whole-platform derivatives. |
| Apache-2.0 / MIT | Broad permissive reuse, including closed derivatives; insufficient alone for the stated copyleft goal. Apache adds explicit patent provisions. |

Sources: [GPLv3 text](https://opensource.org/license/gpl-3.0), [AGPLv3 text](https://opensource.org/license/agpl-3.0), [LGPLv3 text](https://opensource.org/license/lgpl-3-0) and [FSF compatibility discussion](https://www.gnu.org/licenses/license-list.html). GNU individual text/FAQ fetches timed out twice; OSI-hosted texts were available. This limitation does not justify guessing disputed linking or jurisdiction questions.

**Recommendation for approval: GPL-3.0-only for Rivet**, because the stated requirement targets distributed derivatives and does not explicitly demand network-use copyleft. Choose AGPL instead only if the owner affirmatively extends that goal to modified hosted deployments. Recommend matching GPL-3.0-only for owner-maintained Unrivet and Rivet-Bench code intended to combine with Rivet, with separate notices for independent tools/assets. This avoids gratuitous intra-official-mod license differences; it is not a declaration about Unrivet's existing license, which was not inspected. Do not add a broad plugin linking exception unless the owner intentionally permits closed combined mods after legal review.

## Boundaries requiring care

Copyleft does not prohibit commercial use or require publishing every private modification to the world. Ordinary execution alone is not the GPL distribution trigger. Generated code is not automatically GPL merely because Rivet generated it; output that includes covered source/templates may carry obligations from those materials, and models/dependencies may impose separate terms. Avoid a promise that all generated output is unrestricted.

Static Rust linking commonly forms one combined binary; dynamic linking and deeply integrated plugins also require combination analysis. Separate executables using ordinary protocols may remain separate works, but subprocess/RPC/MCP labels are not automatic legal safe harbors: actual coupling, transferred code/data structures and distribution matter. Network use and distribution are different questions. SDK/protocol implementation licenses and the server/tool implementation's license must be assessed separately.

For current Rust dependencies, prior checksum/metadata evidence is not full distribution clearance. Reconcile Cargo.lock against exact source license/NOTICE files, generated/native bundled components and actual features. Preserve alternative license expressions (including exceptions), identify Apache patent/NOTICE requirements, SQLite public-domain provenance, certificate-data terms and rmcp retained-contribution terms. GPLv3 compatibility cannot be inferred from a short “permissive” summary or a repository badge. Reject incompatible combined dependencies or obtain a valid permission route before packaging.

Third-party CLI tools, emulator images, browsers, models, signatures and databases retain independent terms. Bundling may trigger source/notices even when invoking a separate process; merely referencing a tool does not establish redistribution permission. Packaging must distinguish source offer mechanics, install information where required, modifications, notices and executable/source correspondence. Preserve reproducible source/build recipes for distributed binaries. No release/compliance certification is claimed.

## Owner handoff and practical compliance gates

Before adoption decide exact SPDX expression, code/docs/examples/template scope, copyright holders/year policy, contributor inbound terms (DCO or explicit agreement only if chosen), patent/trademark policy and official-mod boundaries. Review whether existing contributors can authorize the chosen license and whether any copied code/assets have different terms. Do not impose a CLA or copyright assignment without approval.

Before distribution inventory all included files/dependencies, generate an SBOM/notices set, verify compatible choices for multi-license packages, include approved license texts and provide corresponding source by a compliant route. Test that packaged source matches binaries and includes modifications/build instructions. Resolve tool/image/model redistribution and source-offer ownership; assign a real compliance contact rather than invent one.

Unresolved legal questions: jurisdiction-specific combination analysis for official/third-party mods and MCP bridges; exact rmcp/transitive native terms; model/output/template rights; AGPL network expectations if hosted use is later chosen; contributor consent for future relicensing; and whether a narrowly scoped exception is desirable. Licensing pending is compatible with honest development documentation; it is not an open-source release grant.
