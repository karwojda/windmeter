package ProjectDescription {
    doc /*
     * Renders a project-description document straight from the
     * requirements/architecture model, via OpenSysML's document queries --
     * so it can't drift from requirements.sysml/system.sysml the way a
     * hand-written summary would. See PROJECT_PRINCIPLES.md.
     *
     * Render: sysml requirements/model -render-document \
     *   ProjectDescription::ProjectDescriptionDoc -o project_description.md
     */

    private import DocumentQueries::*;
    private import KerML::Root::Element;
    private import ScalarValues::*;
    private import WindMeterDomain::*;
    private import WindMeterRequirements::*;
    private import WindMeterSystem::*;

    // Requirement usages (REQ-001 etc.) carry the short ID; their
    // documentation lives on the requirement def they're typed by (see
    // requirements.sysml) -- two different elements, so this renders as
    // two views rather than one merged table: a compact index here, and
    // each requirement's full text under ReqDescriptions below. Each
    // query below is document-specific (always this model's
    // WindMeterRequirements/WindMeterUnit), so the root is a fixed
    // `Named(...)` lookup rather than an `in` parameter a caller binds --
    // a query parameter can only bind a feature (a part/requirement
    // usage), not a package or definition name directly.
    calc def ReqIndex :> Query {
        Project(
            source = OrderBy(
                source = WhereName(
                    source = WhereType(
                        source = Descendants(source = Named(qualifiedName = "WindMeterRequirements"), maxDepth = 1),
                        type = "RequirementUsage"
                    ),
                    operator = "!=", value = ""
                ),
                property = "shortName", direction = "ascending", missing = "last", multiple = "error"
            ),
            properties = ("shortName", "name")
        )
    }

    // The requirement defs, reached by following each usage's "typing"
    // relationship to what it's typed by -- this is where the doc text
    // actually lives.
    calc def ReqDescriptions :> Query {
        Project(
            source = RelatedElements(
                source = WhereType(
                    source = Descendants(source = Named(qualifiedName = "WindMeterRequirements"), maxDepth = 1),
                    type = "RequirementUsage"
                ),
                relationshipKind = "typing", direction = "outgoing", maxDepth = 1
            ),
            properties = ("name", "documentation")
        )
    }

    // Every requirement and what satisfies it -- in this model, always
    // windMeterV1, the one built unit; still worth showing, since an
    // unsatisfied requirement would show up here as an empty cell.
    calc def TraceMatrix :> Query {
        Project(
            source = OrderBy(
                source = WhereName(
                    source = WhereType(
                        source = Descendants(source = Named(qualifiedName = "WindMeterRequirements"), maxDepth = 1),
                        type = "RequirementUsage"
                    ),
                    operator = "!=", value = ""
                ),
                property = "shortName", direction = "ascending", missing = "last", multiple = "error"
            ),
            properties = ("shortName", "name"),
            columns = (
                RelatedColumn(name = "satisfiedBy", relationshipKind = "satisfaction", direction = "incoming", maxDepth = 1)
            )
        )
    }

    // WindMeterUnit's six parts (the architecture) with their
    // documentation, reached the same way as ReqDescriptions -- rooted at
    // the generic unit definition, not windMeterV1, since windMeterV1
    // only *redeclares* the one part it swaps (windSensor -> the
    // Davis-6410-based CupAndVaneWindSensor); its other five parts are
    // inherited, not owned, so they aren't direct descendants of
    // windMeterV1 itself (see v1Note below for the one override).
    calc def SystemComponents :> Query {
        Project(
            source = RelatedElements(
                source = WhereType(
                    source = Descendants(source = Named(qualifiedName = "WindMeterSystem::WindMeterUnit"), maxDepth = 1),
                    type = "PartUsage"
                ),
                relationshipKind = "typing", direction = "outgoing", maxDepth = 1
            ),
            properties = ("name", "documentation")
        )
    }

    part def ProjectDescriptionDoc :> Document {
        attribute redefines title = "windmeter: Project Description";

        part intro : Paragraph {
            attribute redefines text = "windmeter is a boat-mounted instrument that senses apparent wind, reads GPS, computes true wind, logs both locally, and broadcasts the result on the boat's NMEA2000 instrument bus. This document is generated from requirements/model/*.sysml via OpenSysML's document queries (sysml -render-document) -- every table below reads the live model, not a hand-maintained copy of it.";
        }

        part domainNote : Paragraph {
            attribute redefines text = "True wind is derived from apparent wind and the boat's own GPS velocity over ground (the classic wind triangle: trueWind = apparentWind - velocityOverGround), rather than a heading sensor or paddlewheel log -- accurate only absent current and leeway, a deliberate first-iteration simplification (see WindMeterDomain::Boat and REQ-002's documented open item below).";
        }

        part requirements : Section {
            attribute redefines title = "Requirements";

            part index : Table {
                attribute redefines caption = "Requirement index";
                calc rows : ReqIndex;
            }

            part descriptions : Definitions {
                attribute redefines term = "name";
                attribute redefines description = "documentation";
                calc rows : ReqDescriptions;
            }
        }

        part architecture : Section {
            attribute redefines title = "System Architecture";

            part intro : Paragraph {
                attribute redefines text = "WindMeterUnit's six parts, each swappable for another implementation of the same role (e.g. REQ-004's ultrasonic-sensor swap-in) without touching the rest:";
            }

            part components : Definitions {
                attribute redefines term = "name";
                attribute redefines description = "documentation";
                calc rows : SystemComponents;
            }

            part v1Note : Paragraph {
                attribute redefines text = "The first hardware build, windMeterV1, uses CupAndVaneWindSensor (a Davis Instruments 6410 anemometer/vane) as its concrete windSensor -- see hardware-sourcing.md for the ruggedness comparison against the sensors not chosen.";
            }
        }

        part traceability : Section {
            attribute redefines title = "Traceability";

            part matrix : Table {
                attribute redefines caption = "Every requirement and what satisfies it";
                calc rows : TraceMatrix;
            }
        }
    }
}
