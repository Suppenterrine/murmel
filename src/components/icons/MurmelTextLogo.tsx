/* eslint-disable i18next/no-literal-string -- Wortmarke: der Produktname wird nicht übersetzt. */

/**
 * Murmel-Wortmarke.
 *
 * Bewusst als SVG-Text gesetzt (statt als Pfade), damit der Schriftzug ohne
 * fremde Markenassets auskommt und über `currentColor` dem Theme folgt.
 * Die viewBox bleibt beim Seitenverhältnis der bisherigen Wortmarke, damit
 * aufrufende Layouts (`width={120}` / `width={200}`) unverändert passen.
 */
const MurmelTextLogo = ({
  width,
  height,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => {
  return (
    <svg
      width={width}
      height={height}
      className={className}
      viewBox="0 0 930 328"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      role="img"
      aria-label="Murmel"
    >
      <text
        x="0"
        y="242"
        fill="currentColor"
        fontFamily="ui-rounded, 'SF Pro Rounded', 'Segoe UI', Inter, system-ui, sans-serif"
        fontSize="240"
        fontWeight="700"
        letterSpacing="-6"
      >
        Murmel
      </text>
    </svg>
  );
};

export default MurmelTextLogo;
