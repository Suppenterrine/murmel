/**
 * Murmel-Markenzeichen: eine Murmel (Glaskugel) mit eingeschlossenem Farbband.
 *
 * Einfarbig gezeichnet über die Theme-Klassen `fill-text`/`stroke-text`, damit
 * das Icon in Hell- und Dunkelmodus gleichermassen funktioniert.
 */
const MurmelHand = ({
  width,
  height,
}: {
  width?: number | string;
  height?: number | string;
}) => (
  <svg
    width={width || 126}
    height={height || 135}
    viewBox="0 0 126 135"
    className="fill-text stroke-text"
    xmlns="http://www.w3.org/2000/svg"
  >
    {/* Aussenkontur der Murmel */}
    <circle
      cx="63"
      cy="67.5"
      r="52"
      fill="none"
      strokeWidth="7"
      strokeLinecap="round"
    />
    {/* Eingeschlossenes Band, das der Glaskugel ihre Tiefe gibt */}
    <path
      d="M25 48c14 14 30 21 48 21s34-7 48-21"
      fill="none"
      strokeWidth="7"
      strokeLinecap="round"
    />
    <path
      d="M25 87c14-14 30-21 48-21"
      fill="none"
      strokeWidth="7"
      strokeLinecap="round"
    />
    {/* Glanzlicht */}
    <circle cx="45" cy="47" r="8" stroke="none" />
  </svg>
);

export default MurmelHand;
