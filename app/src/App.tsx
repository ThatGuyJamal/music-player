import {useState} from "react";

export default function () {
    const [count, setCount] = useState(0)

    return (
        <div>
            <h1 className={"text-red-500"}>Phenix Music Box</h1>
            <button onClick={() => {
                setCount(count + 1)
            }}>The count is {count}</button>
        </div>
    );
}