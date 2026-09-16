export function BackupInstructions() {
  return (
    <section className="notice" aria-label="Expected backup format">
      <b>For complete Rates and PID inspection: Betaflight CLI dump all</b>
      <ol>
        <li>
          Connect the configured flight controller to Betaflight Configurator
          and open the CLI tab.
        </li>
        <li>
          Type <code>dump all</code> and press Enter. Wait for the output to
          finish.
        </li>
        <li>
          Save the entire output as a text file, including the firmware header
          and all profiles, then open it here. You can also paste the entire
          output.
        </li>
      </ol>
      <p>
        This includes unchanged settings. A <code>diff</code> or{" "}
        <code>diff all</code> backup is accepted, but omitted Rates, Expo, or
        PID values may remain unknown. No reset or <code>defaults</code> command
        is needed. FlightLens currently supports Betaflight 4.2, 4.3, 4.4, 4.5,
        and 2025.12 schemas; a full dump does not add support for other firmware
        versions.
      </p>
    </section>
  );
}

export function MissingSettingsNotice() {
  return (
    <>
      <p className="notice" role="status">
        <b>Incomplete inspection data.</b> Unknown means this backup does not
        establish the value; it does not mean zero. For complete inspection,
        capture
        <code> dump all</code> as described below. Omitted vendor defaults
        cannot be reconstructed reliably from a diff.
      </p>
      <BackupInstructions />
    </>
  );
}
