import { avatarTone, initials } from "../lib/avatar";

type AvatarProps = {
  id: string;
  displayName: string;
  userName: string;
  compact: boolean;
};

export function Avatar({ id, displayName, userName, compact }: AvatarProps) {
  return (
    <span
      aria-hidden="true"
      data-tone={avatarTone(id)}
      className={`avatar grid shrink-0 place-items-center rounded-full font-semibold ${
        compact ? "size-8 text-[12px]" : "size-10 text-[14px]"
      }`}
    >
      {initials(displayName, userName)}
    </span>
  );
}
