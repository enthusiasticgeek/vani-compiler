# Contributor License Agreement — vāṇī Compiler

**Project**: vāṇī compiler (`vani-compiler`)
**Maintainer**: Pratik M. Tambe &lt;enthusiasticgeek@gmail.com&gt;
**Agreement version**: 1.2 — 2026-07-10

> **Note**: This agreement was drafted in good faith but has not been reviewed
> by a licensed attorney. Both parties should seek independent legal counsel
> for any serious legal matter.

---

## 1. Definitions

**"You"** (or **"Your"**) means the individual person or legal entity making a Contribution.

**"Contribution"** means any original work of authorship, including any modification or addition to an existing work, intentionally submitted by You to the Project in any form — source code, documentation, tests, examples, or other material.

**"Project"** means the vāṇī compiler source code, documentation, and associated materials maintained at <https://github.com/enthusiasticgeek/vani-compiler>.

**"Maintainer"** means Pratik M. Tambe, the copyright holder and primary maintainer of the Project.

**"Approved Contributor"** means a person whose GitHub username appears in [`CONTRIBUTORS_APPROVED.md`](CONTRIBUTORS_APPROVED.md) following explicit written approval by the Maintainer.

---

## 2. Contributor Access — Application and Approval

**Contribution access is not automatic.** Pull requests from unapproved GitHub usernames will not be merged regardless of content quality.

### Step 1 — Open a Contributor Application Issue

Open a **GitHub Issue** in the `vani-compiler` repository with the title:

```
[CLA] Contributor application — @your-github-username
```

Include the following in the issue body:

```
### Contributor Application

**GitHub username**: @[your-github-username]
**Full legal name**: [Your Full Legal Name]
**Email**: [your@email.com]
**Affiliation** (employer / university / independent): [affiliation or "Independent"]

### Relevant credentials / background
[Describe your relevant experience: systems programming, compilers, Rust, LLVM,
 language design, formal verification, or other applicable skills. Link to
 public work (GitHub profile, papers, prior open-source contributions) that
 demonstrates relevant expertise.]

### Intended contribution area(s)
[Describe what you intend to contribute: bug fixes, new language features,
 backends, documentation, benchmarks, test cases, etc.]

### Employment and contractual independence declaration
**Current employer (or "Self-employed" / "Student" / "Unemployed")**:
[Name of employer, or status]

**Does your employer operate in a field related to compilers, programming
languages, developer tooling, or systems software?** (yes / no / N/A):
[answer]

**Have you reviewed your employment contract, IP assignment agreement,
non-compete, moonlighting clause, or any other contractual obligation that
could apply to this Contribution?** (yes / no / N/A):
[answer]

**Do any of those agreements restrict or prohibit this Contribution?**
(yes / no — if yes, explain or do not apply):
[answer]

**If your employer could claim any rights to this Contribution** (e.g.,
made on company time, company equipment, or in a field related to your
employment), have you obtained written employer permission? (yes / N/A):
[answer or "N/A — contribution is entirely independent of my employment"]

### CLA Declaration
I have read the vāṇī Compiler Contributor License Agreement (CLA.md v1.2)
and agree to its terms, including the Employment Independence and
Indemnification provisions (§6).

I declare that the employment information above is accurate and complete.
I understand that any legal issues arising from my violation of employment
or contractual obligations are solely my responsibility, and I agree to
defend and indemnify the Maintainer and Project against any resulting claims.

Signed: [Your Full Legal Name]
Date:   [YYYY-MM-DD]
```

### Step 2 — Maintainer Review

The Maintainer will review your GitHub profile, credentials, and stated contribution area. Review may take up to two weeks. The Maintainer may:

- **Approve**: Your username is added to [`CONTRIBUTORS_APPROVED.md`](CONTRIBUTORS_APPROVED.md) and you receive a written approval comment on the Issue.
- **Request more information**: The Maintainer may ask follow-up questions before making a decision.
- **Decline**: The Maintainer may decline any application at sole discretion, with or without explanation.

Approval is not permanent — it may be revoked at any time (see §9).

### Step 3 — Open Pull Requests

Once approved, you may open pull requests. Reference your approval issue number in the first PR (e.g., "CLA approved in #42").

---

## 3. Copyright License Grant

Subject to the terms of this Agreement (and in particular subject to the representations and indemnification in §§5–6), You hereby grant to the Maintainer, and to all recipients of software distributed by the Maintainer, a **perpetual, worldwide, non-exclusive, no-charge, royalty-free, irrevocable** copyright license to:

- reproduce, prepare derivative works of, publicly display, publicly perform, sublicense, and distribute Your Contributions and any derivative works thereof.

---

## 4. Patent License Grant

Subject to the terms of this Agreement (and in particular subject to the representations and indemnification in §§5–6), You hereby grant to the Maintainer, and to all recipients of software distributed by the Maintainer, a **perpetual, worldwide, non-exclusive, no-charge, royalty-free, irrevocable** (except as stated in §4a) patent license to make, have made, use, offer to sell, sell, import, and otherwise transfer the Project and any derivative works, where such license applies only to those patent claims licensable by You that are necessarily infringed by Your Contribution(s) alone or in combination with the Project.

This license extends to patent claims held or controlled by Your employer or any entity on whose behalf You are making the Contribution, to the extent such claims are necessarily infringed by Your Contribution.

---

## 4a. Patent Non-Assertion Covenant

**You covenant, on behalf of yourself and your employer, successors, and assigns, not to assert any patent claim against the Project, the Maintainer, or any recipient or user of the Project's software**, where such patent claim:

- (a) covers an invention first disclosed in this Project (see [`PRIOR_ART.md`](PRIOR_ART.md));
- (b) covers an invention implemented in, substantially derived from, or functionally equivalent to Your Contribution or any existing feature of the Project at the time of Your Contribution; or
- (c) would, if asserted, prevent or restrict any person from using, distributing, modifying, or building upon the Project under its existing open-source license.

This covenant runs with Your Contribution and is binding on You, Your employer, and any successor or assignee of Your patent rights. You represent that You have the authority to make this covenant on behalf of Your employer with respect to patents that cover Your Contribution.

**You further covenant not to file, fund, or assist any patent application** on any invention first disclosed in [`PRIOR_ART.md`](PRIOR_ART.md) or implemented in the Project at the time of Your Contribution.

---

## 4b. Patent Aggression Termination

If You, Your employer, or any entity acting on Your behalf or to which You have assigned any interest:

- institutes a patent infringement proceeding (including a cross-claim or counterclaim) against the Project, the Maintainer, or any downstream recipient of the Project's software; or
- funds, controls, or directs any third party to make such an assertion;

then **all patent licenses granted under §4, all copyright licenses granted under §3, and all contributor approvals granted under §2 shall terminate automatically** as of the date such proceeding or assertion is initiated. No notice is required for this termination to take effect.

Termination under this section does not relieve You of indemnification obligations under §6.2 for Contributions already incorporated into the Project.

---

## 5. Your Representations

By submitting a Contribution application and any subsequent Contributions, You represent that:

1. **Entitlement** — You are legally entitled to grant the licenses in §§3–4.

2. **Employment and contractual clearance** — You have reviewed every employment agreement, contractor agreement, IP assignment clause, moonlighting or outside-activity restriction, non-compete agreement, non-disclosure agreement, and any other contractual or fiduciary obligation binding on You, and You confirm that:
   - making this Contribution does **not** violate any such obligation;
   - Your employer (if any) does **not** hold, claim, or could not reasonably claim ownership of this Contribution by virtue of your employment relationship (e.g., work-for-hire, invention-assignment, or scope-of-employment doctrines); and
   - if Your employer operates in a field related to compilers, programming languages, developer tooling, or systems software, You have either (a) obtained written permission from Your employer to make this Contribution, or (b) confirmed in writing (in your application) that the Contribution is entirely independent of your employment duties, created on your own time, using your own equipment, and not in a field covered by your IP assignment clause.

3. **Original authorship** — Each Contribution is Your original creation, unless disclosed as required by §7.

4. **Third-party notices** — You have disclosed all third-party licenses, patents, or restrictions of which You are aware that are associated with any part of Your Contribution.

5. **No litigation** — You are not aware of any pending or threatened patent, copyright, or employment-related claim that would prevent the Project from distributing Your Contribution under its existing license.

6. **Accurate identity and disclosure** — The GitHub account used to apply is under Your sole control and accurately identifies You or Your organisation. The employment information provided in Your application is accurate and complete. Applications made under false identity or with materially false employment disclosures are void and constitute grounds for immediate revocation and legal action.

---

## 6. Employment Independence and Indemnification

### 6.1 Sole responsibility for employment compliance

**Your compliance with your own employment agreements is entirely your responsibility.** The Maintainer and the Project:

- have no means to verify the terms of Your employment, contractor, or non-compete agreements;
- do not undertake to review or advise on those agreements;
- are **not liable** for any consequences arising from Your decision to contribute in violation of those agreements; and
- will not modify or remove accepted Contributions solely on the basis of a third-party employment-related claim, except where legally compelled to do so.

If a dispute arises between You and Your employer over ownership of a Contribution, that dispute is between You and Your employer. The Project will cooperate with lawfully issued court orders but will not voluntarily withdraw accepted Contributions on the basis of unverified employment claims.

### 6.2 Indemnification

**You agree to defend, indemnify, and hold harmless** the Maintainer (Pratik M. Tambe), and all downstream recipients of the Project's software, from and against any and all third-party claims, demands, actions, proceedings, damages, losses, liabilities, costs, and expenses (including reasonable attorneys' fees) arising out of or related to:

- (a) any breach or alleged breach by You of any employment agreement, contractor agreement, IP assignment clause, moonlighting or outside-activity restriction, non-compete agreement, or other contractual obligation;
- (b) any claim by Your employer, former employer, or any other third party that Your Contribution was made in violation of their intellectual property rights by reason of Your employment relationship or contractual obligations;
- (c) any material misrepresentation or omission in Your Contributor Application regarding Your employment status, contractual obligations, or the ownership of Your Contribution; or
- (d) any false or misleading statement made in the CLA Declaration.

This indemnification obligation survives termination or revocation of Your contributor status and continues for as long as the Contribution remains part of the Project or any derivative work.

### 6.3 Notification obligation

If, at any time after Your application is approved, You become aware of any employment, contractual, or legal restriction that could affect the validity of Your representations in §5 or Your ability to contribute, You must **notify the Maintainer in writing within 14 days** at &lt;enthusiasticgeek@gmail.com&gt;. Failure to notify is itself a material breach of this Agreement.

### 6.4 Employer-authorized contributions

If Your employer has authorized Your contributions and holds joint or contingent rights to Your Contribution, Your application must include written evidence of that authorization (e.g., a signed email from an authorized officer of Your employer confirming the grant of permission). Both You and an authorized representative of Your employer must sign the CLA Declaration.

---

## 7. Third-Party Materials

If Your Contribution includes material authored by others, You must:

- Have the right to submit that material under the MIT License or a compatible license; and
- Clearly identify all such material in the pull request body.

---

## 8. Forking Policy

The vāṇī compiler is published under the MIT License, which permits forking and redistribution. The Maintainer **does not technically prevent forks** of this repository. However:

- **Forks for personal study or experimentation** are welcome and require no approval.
- **Forks intended to redistribute a modified compiler** must retain the original MIT copyright notice and `LICENSE` file as required by the MIT License.
- **Forks intended to create a competing compiler, package registry, or commercial product** based substantially on this codebase are discouraged but are within the terms of the MIT License. The Maintainer requests (but cannot require) that you contact &lt;enthusiasticgeek@gmail.com&gt; to discuss coordination before public release.
- **Contributions back to this repository** always require the approval process in §2 regardless of fork status.

The Maintainer reserves the right to change the license for future releases. Past releases remain under the license in effect at the time of release.

---

## 9. Revocation and Enforcement

Contributor approval may be revoked by the Maintainer at any time. Revocation
removes the contributor's GitHub username from
[`CONTRIBUTORS_APPROVED.md`](CONTRIBUTORS_APPROVED.md) and, for permanent bans,
adds it to [`governance.json`](governance.json) → `blacklisted`. Pull requests
from revoked contributors will not be merged. Revocation does not relieve a
former contributor of indemnification obligations under §6.2 for Contributions
already accepted.

### 9.1 Violation categories

Violations fall into two tiers:

**Tier 1 — Immediate permanent revocation (no warning):**

| Code | Violation |
|------|-----------|
| T1-A | **Malicious contribution** — deliberately introducing malware, backdoors, cryptominers, or destructive payloads into the Project |
| T1-B | **False identity or material misrepresentation** — application or Contributions made under a false GitHub identity, or with materially false employment or IP ownership disclosures (§5.6) |
| T1-C | **Critical security sabotage** — knowingly introducing or concealing a critical, actively-exploited security vulnerability |
| T1-D | **Patent aggression** — initiating patent proceedings against the Project, the Maintainer, or any downstream recipient (automatic termination under §4b also applies) |
| T1-E | **CSAM or illegal content** — submitting any material that violates applicable law |

**Tier 2 — Warning → cure period → revocation:**

| Code | Violation |
|------|-----------|
| T2-A | **Unaddressed security vulnerability** — §6.3 notification obligation not met within 14 days of written notice |
| T2-B | **Repeated harmful conduct** — sustained harassment, hostile communication, or bad-faith review activity |
| T2-C | **License fraud** — contributing code under a claimed license that the contributor does not hold rights to grant |
| T2-D | **Employment non-disclosure** — failure to notify the Maintainer of a new employment restriction that could affect the validity of a Contribution (§6.3) |
| T2-E | **Extended inactivity** — no accepted Contributions for 24+ consecutive months (approval lapses; reapplication required) |

### 9.2 Standard enforcement process (Tier 2)

1. **Notice** — The Maintainer opens a GitHub issue in `vani-compiler` and
   @-mentions the contributor with the violation code, the affected
   Contribution(s), the required corrective action, and a **14-day cure
   deadline**.
2. **Cure period** — The contributor must take the required action within the
   deadline (amend the PR, provide missing documentation, contact the Maintainer,
   etc.).
3. **Escalation** — If the cure period expires without action:
   - First offence: 90-day suspension (username removed from `CONTRIBUTORS_APPROVED.md`;
     reapplication required after the suspension period).
   - Second offence: permanent revocation and blacklist entry in `governance.json`.
4. **Resolution** — If the contributor cures within the deadline, the issue is
   closed as resolved. No suspension is applied for a first-time cure.

### 9.3 Immediate action process (Tier 1)

1. The Maintainer immediately removes the username from `CONTRIBUTORS_APPROVED.md`
   and adds it to `governance.json → blacklisted` with the violation category
   and date.
2. A GitHub issue is opened in `vani-compiler` to notify the contributor and
   record the decision publicly.
3. The revocation is permanent unless successfully appealed (§9.4).

### 9.4 Appeal

A revoked contributor (Tier 1 or Tier 2 permanent) may appeal **once** within
**14 days** of the revocation date by opening a GitHub issue in `vani-compiler`
with:

- **Title**: `[Revocation appeal] @your-github-username`
- **Body**: specific grounds (factual error, account compromise, or — for Tier 2
  permanent only — full remediation with evidence that recurrence is prevented)

The Maintainer will post a decision within 14 days of the appeal issue being
opened.

| Ground | Applies to |
|--------|-----------|
| Factual error — wrong account identified, or violation misattributed | All tiers |
| Account compromise — violation committed by an attacker; contributor can demonstrate this | All tiers |
| Full remediation — violation fully corrected, recurrence prevented | Tier 2 permanent only; not available for T1-A, T1-B, T1-C, or T1-E |

If an appeal is upheld, the username is moved back to a pending state and the
contributor must reapply (§2) before being reinstated to `CONTRIBUTORS_APPROVED.md`.

---

## 10. No Warranty

Your Contributions are provided **"AS IS"**, without warranty of any kind, express or implied.

---

## 11. Project License and Future Relicensing

Accepted Contributions will be distributed as part of the Project under the **MIT License** (see [`LICENSE`](LICENSE)). The Maintainer reserves the right to relicense the Project for future releases (e.g., dual-license for commercial use). Your grants under §§3–4 permit such relicensing. This Agreement does not grant You any right to use the Maintainer's name, trademarks, or project branding in a way that implies endorsement of Your own products or services.

---

## 12. Governing Law and Jurisdiction

**Intellectual property matters** (patents, copyright, trade secrets) arising
from this Agreement are governed by **United States federal law**, including
the Patent Act (35 U.S.C.), the Copyright Act (17 U.S.C.), and the Defend
Trade Secrets Act (18 U.S.C. § 1836). Federal courts of the United States
have exclusive jurisdiction over such matters.

**Contract and other disputes** are governed by the laws of **the state in
which the Maintainer is domiciled at the time the dispute is initiated**. If
the Maintainer is not domiciled in the United States at that time, the parties
agree that the **State of California** (chosen because the Project is hosted on
GitHub, Inc., a California company, and California has strong open-source
contributor protections) governs, and the state and federal courts located in
the Northern District of California shall have exclusive jurisdiction.

**Dispute resolution sequence**:
1. **Written negotiation** — the disputing party notifies the Maintainer in
   writing; parties have 30 days to resolve in good faith.
2. **Mediation** — if unresolved, the parties agree to attempt mediation
   before a mutually agreed neutral mediator for 30 additional days.
3. **Binding arbitration** — if mediation fails, disputes shall be resolved
   by binding arbitration under the rules of JAMS (or, if the claim is under
   USD $10,000, by small claims court in the Maintainer's jurisdiction).
4. **Litigation** — litigation is a last resort. The prevailing party in any
   litigation is entitled to reasonable attorneys' fees.

**Patent matters** may be brought directly to federal court without first
exhausting steps 1–3, given the time-sensitive nature of patent proceedings
(e.g., inter partes review deadlines).

> The Maintainer is domiciled in the United States. US jurisdiction is
> appropriate because the Project was created and is maintained in the US
> and is hosted on GitHub, Inc. (a US company).

---

## 13. Export Compliance

You represent that Your Contribution does not originate from, and will not be
submitted by any person in, a country subject to US export controls or
sanctions (including but not limited to OFAC-sanctioned jurisdictions). You
are not listed on any US government denied-party list (Entity List, SDN List,
etc.).

---

## 14. Miscellaneous

**Severability**: If any provision of this Agreement is held unenforceable, it
shall be modified to the minimum extent necessary to make it enforceable; all
other provisions remain in full force.

**Entire Agreement**: This Agreement, together with [`PATENTS.md`](PATENTS.md)
and [`CONTRIBUTORS_APPROVED.md`](CONTRIBUTORS_APPROVED.md), constitutes the
entire agreement between You and the Maintainer regarding Your Contributions
and supersedes all prior discussions.

**No Waiver**: The Maintainer's failure to enforce any provision does not
constitute a waiver of the right to enforce it in the future.

**Modification**: The Maintainer may publish revised versions of this CLA. Each
version is identified by a version number. Your Contributions made after a new
version is published are governed by the new version. Contributions already
accepted under a prior version remain governed by that version.

**Assignment**: The Maintainer may assign this Agreement in connection with a
merger, acquisition, or sale of substantially all assets of the Project. You
may not assign your rights under this Agreement without written consent.

---

## Corporate Contributors

If You are making Contributions on behalf of a legal entity, a duly authorised representative must submit the application Issue using the following form in addition to the standard fields:

```
### Corporate authorization
I, [Authorized Signatory Name], [Title], am authorized to sign this Agreement
on behalf of [Legal Entity Name] and accept the terms of the vāṇी Compiler
Contributor License Agreement (CLA.md v1.1) on its behalf.

Entity:  [Legal Entity Name]
Signed:  [Authorized Signatory Name]
Email:   [contact@entity.example.com]
GitHub:  @[entity-github-org-or-user]
Date:    [YYYY-MM-DD]
```

---

## Questions

Open a GitHub Discussion in the `vani-compiler` repository, or contact the Maintainer directly at &lt;enthusiasticgeek@gmail.com&gt;.

---

*Modeled on the Apache Software Foundation Individual Contributor License Agreement v2.0, with an explicit application-and-approval process inspired by the Vāṇī Kosh Publisher Agreement v1.0. Patent non-assertion and aggression-termination clauses modeled on the Open Invention Network License and Apache License v2.0 §3. Revision: 2026-07-10 v1.3.*
