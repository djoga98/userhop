type ToggleProps = {
  id: string;
  label: string;
  description?: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
};

export function Toggle({
  id,
  label,
  description,
  checked,
  disabled = false,
  onChange,
}: ToggleProps) {
  const descriptionId = description ? `${id}-description` : undefined;
  return (
    <div className="flex items-center justify-between gap-4 py-2">
      <span className="min-w-0">
        <label htmlFor={id} className="block text-[13.5px]">
          {label}
        </label>
        {description && (
          <span id={descriptionId} className="block text-[12px] text-secondary">
            {description}
          </span>
        )}
      </span>
      <button
        id={id}
        type="button"
        role="switch"
        aria-checked={checked}
        aria-describedby={descriptionId}
        disabled={disabled}
        onClick={() => {
          onChange(!checked);
        }}
        className={`relative h-5 w-10 shrink-0 rounded-full border transition-colors duration-150 disabled:opacity-40 ${
          checked ? "border-accent bg-accent" : "border-control-strong bg-transparent"
        }`}
      >
        <span
          aria-hidden="true"
          className={`absolute top-1/2 size-3 -translate-y-1/2 rounded-full transition-[left] duration-150 ${
            checked ? "left-[22px] bg-on-accent" : "left-[4px] bg-control-strong"
          }`}
        />
      </button>
    </div>
  );
}
