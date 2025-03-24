const TabNavigation = () => (
    <div className="flex gap-[13px]">
      {["Dashboard", "Profile"].map((tab) => (
        <button
          key={tab}
          className="w-[170px] hidden md:block h-[44px] bg-[#373737] hover:text-[whitesmoke] transition-all duration-200 font-['Work_sans'] font-[300] rounded-t-[48px] text-[#CCCCCC]"
        >
          {tab}
        </button>
      ))}
      
        <select className="w-fit px-2 flex items-center justify-center md:hidden h-[44px] bg-[#373737] text-[#CCCCCC] font-['Work_sans'] font-[300] rounded-[4px]">
          {["Dashboard", "Profile"].map((tab) => (
            <option key={tab} value={tab}>
              {tab}
            </option>
          ))}
        </select>
    </div>
  );
  
  export default TabNavigation;
  