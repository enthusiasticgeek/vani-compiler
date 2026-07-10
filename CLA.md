# Contributor License Agreement — vāṇī Compiler

**Project**: vāṇी compiler (`vani-compiler`)
**Maintainer**: Pratik M. Tambe &lt;enthusiasticgeek@gmail.com&gt;
**Agreement version**: 1.1 — 2026-07-10

> **Note**: This agreement was drafted in good faith but has not been reviewed
> by a licensed attorney. Both parties should seek independent legal counsel
> for any serious legal matter.

---

## 1. Definitions

**"You"** (or **"Your"**) means the individual person or legal entity making a Contribution.

**"Contribution"** means any original work of authorship, including any modification or addition to an existing work, intentionally submitted by You to the Project in any form — source code, documentation, tests, examples, or other material.

**"Project"** means the vāṇी compiler source code, documentation, and associated materials maintained at <https://github.com/enthusiasticgeek/vani-compiler>.

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

### CLA Declaration
I have read the vāṇी Compiler Contributor License Agreement (CLA.md v1.1)
and agree to its terms.

Signed: [Your Full Legal Name]
Date:   [YYYY-MM-DD]
```

### Step 2 — Maintainer Review

The Maintainer will review your GitHub profile, credentials, and stated contribution area. Review may take up to two weeks. The Maintainer may:

- **Approve**: Your username is added to [`CONTRIBUTORS_APPROVED.md`](CONTRIBUTORS_APPROVED.md) and you receive a written approval comment on the Issue.
- **Request more information**: The Maintainer may ask follow-up questions before making a decision.
- **Decline**: The Maintainer may decline any application at sole discretion, with or without explanation.

Approval is not permanent — it may be revoked at any time (see §8).

### Step 3 — Open Pull Requests

Once approved, you may open pull requests. Reference your approval issue number in the first PR (e.g., "CLA approved in #42").

---

## 3. Copyright License Grant

Subject to the terms of this Agreement, You hereby grant to the Maintainer, and to all recipients of software distributed by the Maintainer, a **perpetual, worldwide, non-exclusive, no-charge, royalty-free, irrevocable** copyright license to:

- reproduce, prepare derivative works of, publicly display, publicly perform, sublicense, and distribute Your Contributions and any derivative works thereof.

---

## 4. Patent License Grant

Subject to the terms of this Agreement, You hereby grant to the Maintainer, and to all recipients of software distributed by the Maintainer, a **perpetual, worldwide, non-exclusive, no-charge, royalty-free, irrevocable** (except as stated below) patent license to make, have made, use, offer to sell, sell, import, and otherwise transfer the Project and any derivative works, where such license applies only to those patent claims licensable by You that are necessarily infringed by Your Contribution(s) alone or in combination with the Project.

If any entity institutes patent litigation against You or any other party alleging that Your Contribution constitutes direct or contributory patent infringement, any patent licenses granted to that entity under this Agreement shall terminate as of the date such litigation is filed.

---

## 5. Your Representations

By submitting a Contribution application and any subsequent Contributions, You represent that:

1. **Entitlement** — You are legally entitled to grant the licenses in §§3–4. If Your employer has rights to intellectual property You create, You represent that Your employer has authorized You to make this Contribution on its behalf, or that Your employer has waived such rights for Your Contributions to this Project.
2. **Original authorship** — Each Contribution is Your original creation, unless disclosed as required by §6.
3. **Third-party notices** — You have disclosed all third-party licenses, patents, or restrictions of which You are aware that are associated with any part of Your Contribution.
4. **No litigation** — You are not aware of any pending or threatened patent or copyright claims that would prevent the Project from distributing Your Contribution under its existing license.
5. **Accurate identity** — The GitHub account used to apply is under Your sole control and accurately identifies You or Your organisation. Applications made under false identity are void.

---

## 6. Third-Party Materials

If Your Contribution includes material authored by others, You must:

- Have the right to submit that material under the MIT License or a compatible license; and
- Clearly identify all such material in the pull request body.

---

## 7. Forking Policy

The vāṇī compiler is published under the MIT License, which permits forking and redistribution. The Maintainer **does not technically prevent forks** of this repository. However:

- **Forks for personal study or experimentation** are welcome and require no approval.
- **Forks intended to redistribute a modified compiler** must retain the original MIT copyright notice and `LICENSE` file as required by the MIT License.
- **Forks intended to create a competing compiler, package registry, or commercial product** based substantially on this codebase are discouraged but are within the terms of the MIT License. The Maintainer requests (but cannot require) that you contact &lt;enthusiasticgeek@gmail.com&gt; to discuss coordination before public release.
- **Contributions back to this repository** always require the approval process in §2 regardless of fork status.

The Maintainer reserves the right to change the license for future releases. Past releases remain under the license in effect at the time of release.

---

## 8. Revocation

Contributor approval may be revoked by the Maintainer at any time for:

- Violation of this Agreement (false representations, prohibited content, etc.)
- Conduct that harms the project community or its users
- Inactivity exceeding 24 months with no accepted contributions (approval lapses; reapplication required)
- Any other reason at the Maintainer's sole discretion

Revocation removes the contributor's username from [`CONTRIBUTORS_APPROVED.md`](CONTRIBUTORS_APPROVED.md). Pull requests from revoked contributors will not be merged.

---

## 9. No Warranty

Your Contributions are provided **"AS IS"**, without warranty of any kind, express or implied.

---

## 10. Project License and Future Relicensing

Accepted Contributions will be distributed as part of the Project under the **MIT License** (see [`LICENSE`](LICENSE)). The Maintainer reserves the right to relicense the Project for future releases (e.g., dual-license for commercial use). Your grants under §§3–4 permit such relicensing. This Agreement does not grant You any right to use the Maintainer's name, trademarks, or project branding in a way that implies endorsement of Your own products or services.

---

## 11. Governing Law

This Agreement is governed by applicable law. Any dispute shall first be addressed through good-faith written negotiation between You and the Maintainer. If unresolved after 30 days, the parties agree to binding arbitration before resorting to litigation.

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

*Modeled on the Apache Software Foundation Individual Contributor License Agreement v2.0, with an explicit application-and-approval process inspired by the Vāṇी Kosh Publisher Agreement v1.0. Revision: 2026-07-10 v1.1.*
