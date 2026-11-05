import { Card, Logo, PageTitle } from "../ui";

export default function About() {
  return (
    <>
      <PageTitle>About</PageTitle>
      <Card>
        <div className="flex items-center gap-4">
          <Logo className="w-12" />
          <div>
            <p className="font-display text-xl font-semibold">Monk Mode</p>
            <p className="text-sm text-neutral">Free and open source.</p>
          </div>
        </div>
      </Card>
      <Card title="What the lock can't do">
        <p>
          The goal is to make giving up slow and annoying, not impossible. An
          admin user can still get around it, for example by:
        </p>
        <ul className="list-disc pl-5">
          <li>Booting into Safe Mode and removing the service</li>
          <li>Reinstalling Windows</li>
          <li>Moving the system clock ahead, which ends a lock early</li>
          <li>Using a browser other than Chrome, Edge or Brave</li>
        </ul>
        <p>
          Desktop apps like Spotify and Discord are never blocked or closed.
        </p>
      </Card>
    </>
  );
}
