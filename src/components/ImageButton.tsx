import { useState } from 'react';
import './ImageButton.css';

interface ImageButtonProps {
  defaultSrc: string;
  hoverSrc: string;
  pressedSrc: string;
  alt: string;
  label?: string;        // two-line label shown over the image
  onClick?: () => void;
  disabled?: boolean;
  loading?: boolean;
  progress?: number | null;
  className?: string;
  textColor?: 'white' | 'black';  // spec: Hacked buttons use black text
}

export function ImageButton({
  defaultSrc,
  hoverSrc,
  pressedSrc,
  alt,
  label,
  onClick,
  disabled = false,
  loading = false,
  progress = null,
  className = '',
  textColor = 'white',
}: ImageButtonProps) {
  const [isHovered, setIsHovered] = useState(false);
  const [isPressed, setIsPressed] = useState(false);

  const isInteractive = !disabled && !loading;
  const currentSrc = isInteractive && isPressed
    ? pressedSrc
    : isInteractive && isHovered
    ? hoverSrc
    : defaultSrc;

  const displayLabel = loading
    ? progress !== null
      ? `${progress}%`
      : 'Loading...'
    : label ?? alt;

  return (
    <button
      className={`image-button${disabled || loading ? ' image-button--disabled' : ''} ${className}`}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => { setIsHovered(false); setIsPressed(false); }}
      onMouseDown={() => { if (isInteractive) setIsPressed(true); }}
      onMouseUp={() => setIsPressed(false)}
      onClick={isInteractive ? onClick : undefined}
      title={alt}
      disabled={disabled || loading}
    >
      <img src={currentSrc} alt={alt} draggable={false} />
      <span className="image-button-label" style={{ color: textColor }}>
        {displayLabel.split('\n').map((line, i) => (
          <span key={i} className="image-button-line">{line}</span>
        ))}
      </span>
    </button>
  );
}
