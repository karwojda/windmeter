package WindMeterDomain {
    doc /*
     * Concept-level vocabulary shared by the requirements and architecture
     * models: the physical quantities involved in apparent/true wind
     * computation. Kept deliberately small - just enough for the other
     * packages to refer to a common Wind/Boat vocabulary.
     */

    private import ISQ::*;
    private import ScalarValues::*;
    private import SI::*;
    private import VectorCalculations::*;
    private import VectorValues::*;

    attribute def GeoPosition {
        attribute latitude: Real;
        attribute longitude: Real;
    }

    item def Wind {
        attribute vector: CartesianVectorValue;
        attribute speed: ISQ::SpeedValue;
        attribute direction: ISQ::AngularMeasureValue;
    }

    part def Boat {
        attribute position: GeoPosition;

        // Course/speed over ground, as delivered by the onboard GPS receiver.
        attribute speedOverGround: ISQ::SpeedValue;
        attribute courseOverGround: ISQ::AngularMeasureValue;
        attribute velocityOverGround: CartesianVectorValue;

        item apparentWind: Wind;

        // trueWind = apparentWind - boat's own velocity (classic wind
        // triangle). Using GPS velocity-over-ground rather than a heading
        // sensor/paddlewheel log is a deliberate simplification for the
        // first iteration - see OpenItems in the requirements package.
        item trueWind: Wind {
            attribute :>> vector = apparentWind.vector - velocityOverGround;
        }
    }
}
