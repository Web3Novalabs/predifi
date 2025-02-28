import Github from "@/svg/github";
import Twitter from "@/svg/twitter";

function Footer(){
    return (
      <footer className="px-5 md:px-10 xl:px-[100px]">
        <h2 className="text-xl font-normal border-b pb-3">PrediFi</h2>
        <div className="flex justify-between items-center pb-8 mt-3">
          <ul className="flex justify-between items-center gap-4">
            <li>About</li>
            <li>Contact</li>
            <li>Privacy</li>
            <li>Terms</li>
          </ul>
          <div className="flex gap-4">
            <Github/>
            <Twitter/>
          </div>
        </div>
      </footer>
    );
}
export default Footer;