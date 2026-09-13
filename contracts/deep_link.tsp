// Fiducia HTTPS deep-link intent — TypeSpec peer authority.
//
// This file and contracts/deep_link.schema.json are independent, human-authored
// peers for the same wire contract (DEN-3971, docs/DEEP_LINKS.md). Neither is
// generated from the other. TJSV (ORESoftware/typespec-json-schema-validator)
// emits a comparison-only JSON Schema from this file and fails closed unless it
// agrees with the authored schema. Both files are byte-identical in
// fiducia-cloud/fiducia-desktop.rs and fiducia-cloud/fiducia-flutter; see
// contracts/parity.sha256.
import "@typespec/json-schema";
using TypeSpec.JsonSchema;

namespace Fiducia.DeepLink;

@doc("A syntactically admitted request that still requires AppLifecycleMachine authorization.")
model FiduciaDeepLinkIntent {
  @doc("Deep-link contract version; admission emits exactly 1.")
  version: 1;

  @doc("An admitted link is only ever a request, never an authorization grant.")
  kind: "request_action";

  @doc("The closed allowlist of actions a canonical link may request.")
  action: "rotate-api-key" | "review-reconciliation";

  @doc("The single canonical spelling admission accepts for the requested action.")
  @format("uri")
  @pattern("^https://fiducia\\.cloud/open\\?action=(rotate-api-key|review-reconciliation)$")
  @maxLength(96)
  canonical_url: string;
}
